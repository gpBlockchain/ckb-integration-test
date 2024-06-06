use crossbeam_channel::{Receiver, Sender};
use lru::LruCache;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::ops::{Add, Sub};
use std::sync::{Arc};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::Semaphore;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use tokio::time::sleep as async_sleep;
use crate::utils::maybe_retry_send_transaction_async;
use std::sync::atomic::{AtomicUsize, Ordering};
use ckb_jsonrpc_types::{OutPoint, CellDep as CellDepJson, Script as ScriptJson, JsonBytes};
use ckb_types::core::{TransactionBuilder, TransactionView};
use ckb_sdk::rpc::ckb_indexer::Cell;
use ckb_types::core::EpochNumberWithFraction;
use ckb_types::H256;
use ckb_types::packed::{Byte32, ScriptOpt, OutPoint as OutPointByte, CellDep, CellInput, CellOutput, Script, Byte};
use ckb_types::prelude::{Builder, Entity, Pack};
use ckb_bench::util::since_from_absolute_epoch_number_with_fraction;
use crate::node::Node;
use crate::user::User;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::time::{UNIX_EPOCH};
use ckb_error::Error;
use ckb_bench::dag;
use daggy::{Dag, Walker};
use rand::prelude::StdRng;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RunReport {
    tx_size: usize,
    begin_time: u128,
    end_time: u128,
    avg_delay_ms: usize,
    delay_ms: Vec<usize>,
    tps: Vec<usize>,
    timestamp: Vec<u128>,
    pub sum_tps: usize,
}

pub struct LiveCellProducer {
    users: Vec<User>,
    nodes: Vec<Node>,
    seen_out_points: LruCache<OutPoint, Instant>,
}

impl LiveCellProducer {
    pub fn new(users: Vec<User>, nodes: Vec<Node>) -> Self {
        let n_users = users.len();

        let mut user_unused_max_cell_count_cache = 1;
        // step_by: 20 : using a sampling method to find the user who owns the highest number of cells.
        // seen_out_points lruCache cache size = user_unused_max_cell_count_cache * n_users + 10
        // seen_out_points lruCache: preventing unused cells on the chain from being reused.
        for i in (0..=users.len() - 1).step_by(20) {
            let user_unused_cell_count_cache = users.get(i).expect("out of bound").get_spendable_single_secp256k1_cells(&nodes[0]).len();
            if user_unused_cell_count_cache > user_unused_max_cell_count_cache && user_unused_cell_count_cache <= 10000 {
                user_unused_max_cell_count_cache = user_unused_cell_count_cache;
            }
            crate::debug!("idx:{}:user_unused_cell_count_cache:{}",i,user_unused_cell_count_cache);
        }
        crate::debug!("user max cell count cache:{}",user_unused_max_cell_count_cache);
        let lrc_cache_size = n_users * user_unused_max_cell_count_cache + 10;
        crate::info!("init unused cache size:{}",lrc_cache_size);
        Self {
            users,
            nodes,
            seen_out_points: LruCache::new(lrc_cache_size * 2),
        }
    }

    pub fn run(mut self, live_cell_sender: Sender<Cell>, log_duration: u64) {
        let mut count = 0;
        let mut start_time = Instant::now();
        let mut duration_count = 0;
        let mut fist_send_finished = true;
        loop {
            let current_loop_start_time = Instant::now();
            let min_tip_number = self
                .nodes
                .iter()
                .map(|node| node.rpc_client().get_tip_block_number().unwrap())
                .min()
                .unwrap();
            for user in self.users.iter() {
                let live_cells = user
                    .get_spendable_single_secp256k1_cells(&self.nodes[0])
                    .into_iter()
                    // TODO reduce competition
                    .filter(|cell| {
                        if self.seen_out_points.contains(&cell.out_point) {
                            return false;
                        }
                        if cell.block_number > min_tip_number {
                            return false;
                        }
                        true
                    })
                    .collect::<Vec<_>>();
                for cell in live_cells {
                    self.seen_out_points
                        .put(cell.out_point.clone().into(), Instant::now());
                    let _ignore = live_cell_sender.send(cell);
                    count += 1;
                    duration_count += 1;
                    if Instant::now().duration_since(start_time) >= Duration::from_secs(log_duration) {
                        let elapsed = start_time.elapsed();
                        crate::info!("[LiveCellProducer] producer count: {} ,duration time:{:?} , duration tps:{}", count,elapsed,duration_count*1000/elapsed.as_millis());
                        duration_count = 0;
                        start_time = Instant::now();
                    }
                }
            }
            if fist_send_finished {
                fist_send_finished = false;
                self.seen_out_points.resize(count * 2)
            }
            crate::debug!("[LiveCellProducer] delay:{:?},total producer:{}",current_loop_start_time.elapsed(),count);
        }
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct AddTxParam {
    pub deps: Vec<CellDepJson>,
    pub _type: ScriptJson,
    pub output_data: JsonBytes,
    pub min_fee: u64,
    pub max_fee: u64,
}

impl AddTxParam {
    pub(crate) fn get_output_data(&mut self) -> ckb_types::packed::Bytes {
        // ckb_types::packed::Bytes::from(self.output_data.clone()
        let mut rng = rand::thread_rng();

        let random_seed: u64 = rng.gen_range(1..=100);
        let random_spawns: u32 = rng.gen_range(5..=15);
        let random_writes: u32 = rng.gen_range(3..=60);
        generate_data_graph(random_seed, random_spawns, random_writes, 3).unwrap().as_bytes().pack()
    }
}

impl AddTxParam {
    pub fn new() -> Self {
        Self {
            deps: vec![],
            _type: ScriptJson::default(),
            output_data: Default::default(),
            min_fee: 2000,
            max_fee: 2000,
        }
    }

    pub fn get_cell_deps(&mut self) -> Vec<CellDep> {
        let mut updated_vec: Vec<CellDep> = Vec::new();
        for item in self.deps.iter() {
            updated_vec.push(CellDep::new_builder()
                .out_point(
                    OutPointByte::new(item.out_point.tx_hash.pack(), item.out_point.index.value())
                ).dep_type(ckb_types::core::DepType::from(item.dep_type.clone()).into())
                .build())
        }
        updated_vec
    }
    pub fn get_script_obj(&mut self) -> ScriptOpt {
        // if self._type
        if self._type.code_hash == H256::default() {
            ScriptOpt::default()
        } else {
            Some(Script::new_builder()
                .code_hash(self._type.code_hash.pack())
                .args(self._type.args.clone().into_bytes().pack())
                .hash_type(ckb_types::core::ScriptHashType::from(self._type.hash_type.clone()).into())
                .build()).pack()
        }
    }

    pub fn get_fee(&mut self) -> u64 {
        let mut rng = rand::thread_rng();
        rng.gen_range(self.min_fee..=self.max_fee)
    }
}

pub struct TransactionProducer {
    // #{ lock_hash => user }
    users: HashMap<Byte32, User>,
    cell_deps: Vec<CellDep>,
    n_inout: usize,
    // #{ lock_hash => live_cell }
    live_cells: HashMap<Byte32, Cell>,
    // #{ out_point => live_cell }
    backlogs: HashMap<Byte32, Vec<Cell>>,
    add_tx_param: AddTxParam,
}

impl TransactionProducer {
    pub fn new(users: Vec<User>, cell_deps: Vec<CellDep>, n_inout: usize, add_tx_param: AddTxParam) -> Self {
        let mut users_map = HashMap::new();
        for user in users {
            // To support environment `CKB_BENCH_ENABLE_DATA1_SCRIPT`, we have to index 3
            // kinds of cells
            users_map.insert(
                user.single_secp256k1_lock_script_via_type()
                    .calc_script_hash(),
                user.clone(),
            );
            users_map.insert(
                user.single_secp256k1_lock_script_via_data()
                    .calc_script_hash(),
                user.clone(),
            );
            users_map.insert(
                user.single_secp256k1_lock_script_via_data1()
                    .calc_script_hash(),
                user.clone(),
            );
        }

        Self {
            users: users_map,
            cell_deps,
            n_inout,
            live_cells: HashMap::new(),
            backlogs: HashMap::new(),
            add_tx_param,
        }
    }

    pub fn run(
        mut self,
        live_cell_receiver: Receiver<Cell>,
        transaction_sender: Sender<TransactionView>,
        log_duration: u64,
    ) {
        // Environment variables `CKB_BENCH_ENABLE_DATA1_SCRIPT` and
        // `CKB_BENCH_ENABLE_INVALID_SINCE_EPOCH` are temporary.
        let enabled_data1_script = match ::std::env::var("CKB_BENCH_ENABLE_DATA1_SCRIPT") {
            Ok(raw) => {
                raw.parse()
                    .map_err(|err| crate::error!("failed to parse environment variable \"CKB_BENCH_ENABLE_DATA1_SCRIPT={}\", error: {}", raw, err))
                    .unwrap_or(false)
            }
            Err(_) => false,
        };
        let enabled_invalid_since_epoch = match ::std::env::var("CKB_BENCH_ENABLE_INVALID_SINCE_EPOCH") {
            Ok(raw) => {
                raw.parse()
                    .map_err(|err| crate::error!("failed to parse environment variable \"CKB_BENCH_ENABLE_INVALID_SINCE_EPOCH={}\", error: {}", raw, err))
                    .unwrap_or(false)
            }
            Err(_) => false,
        };
        crate::info!("CKB_BENCH_ENABLE_DATA1_SCRIPT = {}", enabled_data1_script);
        crate::info!(
            "CKB_BENCH_ENABLE_INVALID_SINCE_EPOCH = {}",
            enabled_invalid_since_epoch
        );
        let mut count = 0;
        let mut start_time = Instant::now();
        let mut duration_count = 0;


        let mut tx_cell_deps = self.cell_deps.clone();
        // tx_cell_deps.extend(self.add_tx_param.get_cell_deps());
        tx_cell_deps.splice(..0, self.add_tx_param.get_cell_deps().iter().cloned());


        while let Ok(live_cell) = live_cell_receiver.recv() {
            let lock_hash = ckb_types::packed::Script::from(live_cell.output.lock.clone()).calc_script_hash();

            if let Some(_live_cell_in_map) = self.live_cells.get(&lock_hash) {
                self.backlogs
                    .entry(lock_hash.clone())
                    .or_insert_with(Vec::new)
                    .push(live_cell);
            } else {
                self.live_cells.insert(lock_hash.clone(), live_cell);
                for (hash, backlog_cells) in self.backlogs.iter_mut() {
                    if self.live_cells.len() >= self.n_inout {
                        break;
                    }
                    if !self.live_cells.contains_key(hash) && !backlog_cells.is_empty() {
                        if let Some(backlog_cell) = backlog_cells.pop() {
                            self.live_cells.insert(hash.clone(), backlog_cell);
                        }
                    }
                }
            }

            if self.live_cells.len() >= self.n_inout {
                let mut live_cells = HashMap::new();
                std::mem::swap(&mut self.live_cells, &mut live_cells);

                let since = if enabled_invalid_since_epoch {
                    since_from_absolute_epoch_number_with_fraction(
                        EpochNumberWithFraction::new_unchecked(0, 1, 1),
                    )
                } else {
                    0
                };
                let inputs = live_cells
                    .values()
                    .map(|cell| {
                        CellInput::new_builder()
                            .previous_output(cell.out_point.clone().into())
                            .since(since.pack())
                            .build()
                    })
                    .collect::<Vec<_>>();
                let outputs = live_cells
                    .values()
                    .map(|cell| {
                        // use tx_index as random number

                        let lock_hash = ckb_types::packed::Script::from(cell.output.lock.clone()).calc_script_hash();
                        let tx_index = cell.tx_index.value();
                        let user = self.users.get(&lock_hash).expect("should be ok");
                        match tx_index % 3 {
                            0 => CellOutput::new_builder()
                                .capacity((cell.output.capacity.value() - self.add_tx_param.get_fee()).pack())
                                .lock(user.single_secp256k1_lock_script_via_data())
                                .type_(self.add_tx_param.get_script_obj())
                                .build(),
                            1 => CellOutput::new_builder()
                                .capacity((cell.output.capacity.value() - self.add_tx_param.get_fee()).pack())
                                .lock(user.single_secp256k1_lock_script_via_type())
                                .type_(self.add_tx_param.get_script_obj())
                                .build(),
                            2 => {
                                if enabled_data1_script {
                                    CellOutput::new_builder()
                                        .capacity((cell.output.capacity.value() - self.add_tx_param.get_fee()).pack())
                                        .lock(user.single_secp256k1_lock_script_via_data1())
                                        .type_(self.add_tx_param.get_script_obj())
                                        .build()
                                } else {
                                    CellOutput::new_builder()
                                        .capacity((cell.output.capacity.value() - self.add_tx_param.get_fee()).pack())
                                        .lock(user.single_secp256k1_lock_script_via_data())
                                        .type_(self.add_tx_param.get_script_obj())
                                        .build()
                                }
                            }
                            _ => unreachable!(),
                        }
                    })
                    .collect::<Vec<_>>();
                let outputs_data = live_cells.values().map(|_| self.add_tx_param.get_output_data());
                let raw_tx = TransactionBuilder::default()
                    .inputs(inputs)
                    .outputs(outputs)
                    .outputs_data(outputs_data)
                    .cell_deps(tx_cell_deps.clone())
                    .build();
                // NOTE: We know the transaction's inputs and outputs are paired by index, so this
                // signed way is okay.
                let witnesses = live_cells.values().map(|cell| {
                    let lock_hash = ckb_types::packed::Script::from(cell.output.lock.clone()).calc_script_hash();
                    let user = self.users.get(&lock_hash).expect("should be ok");
                    user.single_secp256k1_signed_witness(&raw_tx)
                        .as_bytes()
                        .pack()
                });
                let signed_tx = raw_tx.as_advanced_builder().witnesses(witnesses)
                    .build();
                crate::debug!("signed tx:{:?}",signed_tx.to_string());
                if transaction_sender.send(TransactionView::from(signed_tx)).is_err() {
                    // SendError occurs, the corresponding transaction receiver is dead
                    return;
                }
                count += 1;
                duration_count += 1;
                if Instant::now().duration_since(start_time) >= Duration::from_secs(log_duration) {
                    let elapsed = start_time.elapsed();
                    crate::info!("[TransactionProducer] producer count: {} liveCell producer remaining :{} ,duration time:{:?}, duration tps:{} ", count,live_cell_receiver.len(),elapsed,duration_count*1000/elapsed.as_millis());
                    duration_count = 0;
                    start_time = Instant::now();
                }
            }
        }
    }
}

pub enum RunType {
    TPS(usize),
    INTERVAL(Duration),
}

pub struct TransactionConsumer {
    nodes: Vec<Node>,
}


impl TransactionConsumer {
    pub fn new(nodes: Vec<Node>) -> Self {
        Self {
            nodes
        }
    }

    pub async fn run(
        &self,
        transaction_receiver: Receiver<TransactionView>,
        max_concurrent_requests: usize,
        t_tx_interval: Duration,
        t_bench: Duration) -> RunReport {
        self.run_internal(transaction_receiver, max_concurrent_requests, RunType::INTERVAL(t_tx_interval), t_bench).await
    }


    pub async fn run_tps(
        &self,
        transaction_receiver: Receiver<TransactionView>,
        max_concurrent_requests: usize,
        tps: usize,
        t_bench: Duration) -> RunReport {
        self.run_internal(transaction_receiver, max_concurrent_requests, RunType::TPS(tps), t_bench).await
    }


    pub async fn run_internal(&self,
                              transaction_receiver: Receiver<TransactionView>,
                              max_concurrent_requests: usize,
                              run_type: RunType,
                              t_bench: Duration) -> RunReport {
        let start_time_stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        let start_time = Instant::now();
        let mut last_log_duration = Instant::now();
        let mut benched_transactions = 0;
        let mut duplicated_transactions = 0;
        let mut delay_ms = vec![];
        let mut tps_vec = vec![];
        let mut timestamp = vec![];
        let mut loop_count = 0;
        let mut i = 0;
        let log_duration_time = 3;
        let mut t_tx_interval = match run_type {
            RunType::TPS(tps) => {
                Duration::from_millis((Duration::from_secs(max_concurrent_requests as u64).as_millis() / (tps as u128 as u128)) as u64)
            }
            RunType::INTERVAL(interval) => {
                interval
            }
        };


        let semaphore = Arc::new(Semaphore::new(max_concurrent_requests));
        let transactions_processed = Arc::new(AtomicUsize::new(0));
        let transactions_total_time = Arc::new(AtomicUsize::new(0));

        let mut pending_tasks = FuturesUnordered::new();

        loop {
            loop_count += 1;
            let tx = transaction_receiver
                .recv_timeout(Duration::from_secs(60 * 3))
                .expect("timeout to wait transaction_receiver");
            i = (i + 1) % self.nodes.len();
            let node = self.nodes[i].clone();
            let permit = semaphore.clone().acquire_owned().await;
            let tx_hash = tx.hash();
            let task = async move {
                let begin_time = Instant::now();
                let result = maybe_retry_send_transaction_async(&node, &tx).await;
                let end_time = begin_time.elapsed();
                if t_tx_interval.as_millis() != 0 {
                    async_sleep(t_tx_interval).await;
                }
                drop(permit);
                (result, tx_hash, end_time)
            };

            pending_tasks.push(tokio::spawn(task));
            while let Some(result) = pending_tasks.next().now_or_never() {
                transactions_processed.fetch_add(1, Ordering::Relaxed);

                let mut use_time = Duration::from_millis(0);

                match result {
                    Some(Ok((Ok(is_accepted), _tx_hash, cost_time))) => {
                        use_time = cost_time;
                        if is_accepted {
                            benched_transactions += 1;
                        } else {
                            duplicated_transactions += 1;
                        }
                    }
                    Some(Ok((Err(err), tx_hash, cost_time))) => {
                        use_time = cost_time;
                        // double spending, discard this transaction
                        crate::info!(
                            "consumer count :{} failed to send tx {:#x}, error: {}",
                            loop_count,
                            tx_hash,
                            err
                        );
                        if !err.contains("TransactionFailedToResolve") {
                            crate::error!(
                                "failed to send tx {:#x}, error: {}",
                                tx_hash,
                                err
                            );
                        }
                    }
                    Some(Err(e)) => {
                        eprintln!("Error in task: {:?}", e);
                    }
                    None => break,
                }
                transactions_total_time.fetch_add(use_time.as_micros() as usize, Ordering::Relaxed);
            }

            if last_log_duration.elapsed() > Duration::from_secs(log_duration_time) {
                let elapsed = last_log_duration.elapsed();
                last_log_duration = Instant::now();
                let duration_count = transactions_processed.swap(0, Ordering::Relaxed);
                let duration_total_time = transactions_total_time.swap(0, Ordering::Relaxed);
                let total_tps = loop_count * 1000 / start_time.elapsed().as_millis() as usize;
                let mut duration_tps = 0;
                let mut duration_delay = 0;
                if duration_count != 0 {
                    duration_delay = duration_total_time / (duration_count as usize);
                    duration_tps = duration_count * 1000000 / (elapsed.as_micros() as usize);
                }
                crate::info!(
                    "[TransactionConsumer] consumer :{} transactions, {} duplicated {} , transaction producer  remaining :{}, log duration {:?} ,duration send tx tps {},duration avg delay {:?},sum tps:{}",
                        loop_count,
                        benched_transactions,
                        duplicated_transactions,
                        transaction_receiver.len(),
                        elapsed,
                        duration_tps,
                        Duration::from_micros(duration_delay as u64),
                        total_tps
                );
                t_tx_interval = match run_type {
                    RunType::TPS(tps) => {
                        dynamic_adjustment_internal(max_concurrent_requests, tps, duration_tps, t_tx_interval, Duration::from_micros(duration_delay as u64))
                    }
                    RunType::INTERVAL(interval) => {
                        interval
                    }
                };
                delay_ms.push(duration_delay);
                tps_vec.push(duration_tps);
                timestamp.push(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis());
            }
            if start_time.elapsed() > t_bench {
                break;
            }
        }
        if delay_ms.len() == 0 {
            return RunReport {
                tx_size: loop_count,
                begin_time: start_time_stamp,
                end_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
                avg_delay_ms: 0,
                delay_ms,
                tps: tps_vec,
                timestamp,
                sum_tps: loop_count * 1000000 / (start_time.elapsed().as_micros() as usize),
            };
        }
        return RunReport {
            tx_size: loop_count,
            begin_time: start_time_stamp,
            end_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
            avg_delay_ms: delay_ms.iter().sum::<usize>() as usize / delay_ms.len() as usize,
            delay_ms,
            tps: tps_vec,
            timestamp,
            sum_tps: loop_count * 1000000 / (start_time.elapsed().as_micros() as usize),
        };
    }
}

/// Calculates and adjusts the internal interval for dynamic adjustment.
///
/// This function takes into account the provided parameters and calculates the
/// appropriate internal interval for achieving the desired Transactions Per Second (TPS).
/// It considers the actual TPS, latest interval, and adjustment interval to determine
/// if the target TPS can be achieved.
///
/// # Arguments
///
/// * `max_concurrent_requests` - The maximum concurrent requests allowed.
/// * `tps` - The target Transactions Per Second (TPS).
/// * `latest_tps` - The most recent TPS measurement.
/// * `latest_adjustment_interval` - The most recent adjustment interval.
/// * `latest_interval` - The most recent interval.
///
/// # Returns
///
/// The adjusted internal interval.
fn dynamic_adjustment_internal(max_concurrent_requests: usize, tps: usize, latest_tps: usize, latest_adjustment_interval: Duration, latest_interval: Duration) -> Duration {

    // Calculate the other cost time = Actual Interval: (1s / Actual TPS) - (Adjusted Interval + Transaction Sending Delay)
    let mut other_cost_time = Duration::from_micros(0);
    if latest_tps != 0 && Duration::from_micros(Duration::from_secs(1).as_micros() as u64 * max_concurrent_requests as u64 / latest_tps as u64) > latest_interval.add(latest_adjustment_interval) {
        other_cost_time = Duration::from_micros(Duration::from_secs(1).as_micros() as u64 * max_concurrent_requests as u64 / latest_tps as u64).sub(latest_interval).sub(latest_adjustment_interval);
    }

    // Maximum Adjustment Interval
    let max_requests_internal = Duration::from_micros((max_concurrent_requests as u64 * 1_000_000) / tps as u64);

    // > Maximum Adjustment Interval
    if latest_interval + other_cost_time > max_requests_internal {
        crate::warn!("Cannot achieve the target TPS");
        return Duration::from_millis(0);
    }

    // < Maximum Adjustment Interval
    let adjusted_interval = max_requests_internal - latest_interval - other_cost_time;
    crate::info!(
            "Max Requests Duration: {:?}, Adjusted Interval: {:?}, Latest Interval: {:?}, Other Cost Time: {:?}",
                max_requests_internal,
                adjusted_interval,
                latest_interval,
                other_cost_time
    );
    adjusted_interval
}


pub fn generate_data_graph(
    seed: u64,
    spawns: u32,
    writes: u32,
    converging_threshold: u32,
) -> Result<dag::Data, Error> {
    let mut rng = StdRng::seed_from_u64(seed);

    let mut spawn_dag: Dag<(), ()> = Dag::new();
    let mut write_dag: Dag<(), ()> = Dag::new();

    // Root node denoting entrypoint VM
    let spawn_root = spawn_dag.add_node(());
    let write_root = write_dag.add_node(());
    assert_eq!(spawn_root.index(), 0);
    assert_eq!(write_root.index(), 0);

    let mut spawn_nodes = vec![spawn_root];
    let mut write_nodes = vec![write_root];

    for _ in 1..=spawns {
        let write_node = write_dag.add_node(());
        write_nodes.push(write_node);

        let previous_node = spawn_nodes[rng.gen_range(0..spawn_nodes.len())];
        let (_, spawn_node) = spawn_dag.add_child(previous_node, (), ());
        spawn_nodes.push(spawn_node);
    }

    let mut write_edges = Vec::new();
    if spawns > 0 {
        for _ in 1..=writes {
            let mut updated = false;

            for _ in 0..converging_threshold {
                let first_index = rng.gen_range(0..write_nodes.len());
                let second_index = {
                    let mut i = first_index;
                    while i == first_index {
                        i = rng.gen_range(0..write_nodes.len());
                    }
                    i
                };

                let first_node = write_nodes[first_index];
                let second_node = write_nodes[second_index];

                if let Ok(e) = write_dag.add_edge(first_node, second_node, ()) {
                    write_edges.push(e);
                    updated = true;
                    break;
                }
            }

            if !updated {
                break;
            }
        }
    }

    // Edge index -> pipe indices. Daggy::edge_endpoints helps us finding
    // nodes (vms) from edges (spawns)
    let mut spawn_ops: HashMap<usize, Vec<usize>> = HashMap::default();
    // Node index -> created pipes
    let mut pipes_ops: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::default();

    let mut spawn_edges = Vec::new();
    // Traversing spawn_dag for spawn operations
    let mut processing = VecDeque::from([spawn_root]);
    while !processing.is_empty() {
        let node = processing.pop_front().unwrap();
        pipes_ops.insert(node.index(), Vec::new());
        let children: Vec<_> = spawn_dag.children(node).iter(&spawn_dag).collect();
        for (e, n) in children.into_iter().rev() {
            spawn_ops.insert(e.index(), Vec::new());
            spawn_edges.push(e);

            processing.push_back(n);
        }
    }

    let mut writes_builder = dag::WritesBuilder::default();
    // Traversing all edges in write_dag
    for e in write_edges {
        let (writer, reader) = write_dag.edge_endpoints(e).unwrap();
        assert_ne!(writer, reader);
        let writer_pipe_index = e.index() * 2 + 1;
        let reader_pipe_index = e.index() * 2;

        // Generate finalized write op
        {
            let data_len = rng.gen_range(1..=1024);
            let mut data = vec![0u8; data_len];
            rng.fill(&mut data[..]);

            writes_builder = writes_builder.push(
                dag::WriteBuilder::default()
                    .from(build_vm_index(writer.index() as u64))
                    .from_fd(build_fd_index(writer_pipe_index as u64))
                    .to(build_vm_index(reader.index() as u64))
                    .to_fd(build_fd_index(reader_pipe_index as u64))
                    .data(
                        dag::BytesBuilder::default()
                            .extend(data.iter().map(|b| Byte::new(*b)))
                            .build(),
                    )
                    .build(),
            );
        }

        // Finding the lowest common ancestor of writer & reader nodes
        // in spawn_dag, which will creates the pair of pipes. Note that
        // all traversed spawn edges will have to pass the pipes down.
        //
        // TODO: we use a simple yet slow LCA solution, a faster algorithm
        // can be used to replace the code here if needed.
        let ancestor = {
            let mut a = writer;
            let mut b = reader;

            let mut set_a = HashSet::new();
            set_a.insert(a);
            let mut set_b = HashSet::new();
            set_b.insert(b);

            loop {
                let parents_a: Vec<_> = spawn_dag.parents(a).iter(&spawn_dag).collect();
                let parents_b: Vec<_> = spawn_dag.parents(b).iter(&spawn_dag).collect();

                assert!(
                    ((parents_a.len() == 1) && (parents_b.len() == 1))
                        || (parents_a.is_empty() && (parents_b.len() == 1))
                        || ((parents_a.len() == 1) && parents_b.is_empty())
                );

                // Update spawn ops to pass down pipes via edges, also update
                // each node's path node list
                if parents_a.len() == 1 {
                    let (_, parent_a) = parents_a[0];
                    set_a.insert(parent_a);

                    a = parent_a;
                }
                if parents_b.len() == 1 {
                    let (_, parent_b) = parents_b[0];
                    set_b.insert(parent_b);

                    b = parent_b;
                }

                // Test for ancestor
                if parents_a.len() == 1 {
                    let (_, parent_a) = parents_a[0];
                    if set_b.contains(&parent_a) {
                        break parent_a;
                    }
                }
                if parents_b.len() == 1 {
                    let (_, parent_b) = parents_b[0];
                    if set_a.contains(&parent_b) {
                        break parent_b;
                    }
                }
            }
        };

        // Update the path from each node to the LCA so we can pass created
        // pipes from LCA to each node
        {
            let mut a = writer;
            while a != ancestor {
                let parents_a: Vec<_> = spawn_dag.parents(a).iter(&spawn_dag).collect();
                assert!(parents_a.len() == 1);
                let (edge_a, parent_a) = parents_a[0];
                spawn_ops
                    .get_mut(&edge_a.index())
                    .unwrap()
                    .push(writer_pipe_index);
                a = parent_a;
            }

            let mut b = reader;
            while b != ancestor {
                let parents_b: Vec<_> = spawn_dag.parents(b).iter(&spawn_dag).collect();
                assert!(parents_b.len() == 1);
                let (edge_b, parent_b) = parents_b[0];
                spawn_ops
                    .get_mut(&edge_b.index())
                    .unwrap()
                    .push(reader_pipe_index);
                b = parent_b;
            }
        }

        // Create the pipes at the ancestor node
        pipes_ops
            .get_mut(&ancestor.index())
            .unwrap()
            .push((reader_pipe_index, writer_pipe_index));
    }

    let mut spawns_builder = dag::SpawnsBuilder::default();
    for e in spawn_edges {
        let (parent, child) = spawn_dag.edge_endpoints(e).unwrap();

        let pipes = {
            let mut builder = dag::FdIndicesBuilder::default();
            for p in &spawn_ops[&e.index()] {
                builder = builder.push(build_fd_index(*p as u64));
            }
            builder.build()
        };

        spawns_builder = spawns_builder.push(
            dag::SpawnBuilder::default()
                .from(build_vm_index(parent.index() as u64))
                .child(build_vm_index(child.index() as u64))
                .fds(pipes)
                .build(),
        );
    }

    let mut pipes_builder = dag::PipesBuilder::default();
    for (vm_index, pairs) in pipes_ops {
        for (reader_pipe_index, writer_pipe_index) in pairs {
            pipes_builder = pipes_builder.push(
                dag::PipeBuilder::default()
                    .vm(build_vm_index(vm_index as u64))
                    .read_fd(build_fd_index(reader_pipe_index as u64))
                    .write_fd(build_fd_index(writer_pipe_index as u64))
                    .build(),
            );
        }
    }

    Ok(dag::DataBuilder::default()
        .spawns(spawns_builder.build())
        .pipes(pipes_builder.build())
        .writes(writes_builder.build())
        .build())
}


fn build_vm_index(val: u64) -> dag::VmIndex {
    let mut data = [Byte::new(0); 8];
    for (i, v) in val.to_le_bytes().into_iter().enumerate() {
        data[i] = Byte::new(*v);
    }
    dag::VmIndexBuilder::default().set(data).build()
}

fn build_fd_index(val: u64) -> dag::FdIndex {
    let mut data = [Byte::new(0); 8];
    for (i, v) in val.to_le_bytes().into_iter().enumerate() {
        data[i] = Byte::new(*v);
    }
    dag::FdIndexBuilder::default().set(data).build()
}
