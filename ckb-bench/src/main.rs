mod logger;
mod node;
mod nodes;
pub mod util;
mod watcher;
mod utils;
mod user;
mod stat;
mod prepare;
mod bench;
mod html;
mod prometheus;
#[cfg(test)]
mod tests;
mod rpc;

use std::fs::File;
use std::io::{Read};
use tokio::runtime::Runtime;
use crate::bench::{AddTxParam, LiveCellProducer, TransactionConsumer, TransactionProducer};
use crate::prepare::{collect, derive_privkeys, dispatch};
use crate::watcher::{Watcher};
use ckb_hash::blake2b_256;
use ckb_sdk::{Address, AddressPayload, NetworkType};
use ckb_types::core::{BlockNumber, DepType, ScriptHashType};
use clap::{value_t_or_exit, values_t_or_exit, App, Arg, ArgMatches, SubCommand, value_t};
use crossbeam_channel::{bounded};
use std::env;
use std::ops::Div;
use std::process::exit;
use std::str::FromStr;
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};
use ckb_types::H256;
use ckb_types::packed::{Byte32, CellDep, OutPoint};
use ckb_types::prelude::{Builder, Entity, Pack};
use url::Url;
use crate::nodes::Nodes;
use crate::user::{SecpLockConfig, User};
use ckb_crypto::secp::{Privkey};
use crate::node::Node;
use crate::html::{generate_html_report, TotalReport, write_to_file};
use crate::prometheus::{MemoryUsageClient, MemoryUsageReport};

#[macro_export]
macro_rules! prompt_and_exit {
    ($($arg:tt)*) => ({
        eprintln!($($arg)*);
        crate::error!($($arg)*);
        ::std::process::exit(1);
    })
}


fn main() {
    let _logger = init_logger();
    entrypoint(clap_app().get_matches());
}

pub fn entrypoint(clap_arg_match: ArgMatches<'static>) {
    match clap_arg_match.subcommand() {
        ("address", Some(arguments)) => {
            let network = network_type_from_argument(
                arguments.value_of("network").expect("network has a default"),
            );
            let owner_privkey = owner_privkey_from_env();
            let lock_config = secp_lock_config_from_arguments(arguments);
            println!(
                "{}",
                address_from_privkey(&owner_privkey, network, lock_config.as_ref())
            );
        }
        ("info", Some(arguments)) => {
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let nodes = rpc_urls
                .iter()
                .map(|url| {
                    Node::init(url.as_str(), url.as_str())
                })
                .collect::<Vec<_>>();
            let n_users = value_t_or_exit!(arguments, "n-users", usize);
            let owner_raw_privkey = env::var("CKB_BENCH_OWNER_PRIVKEY").unwrap_or_else(|err| {
                prompt_and_exit!(
                    "cannot find \"CKB_BENCH_OWNER_PRIVKEY\" from environment variables, error: {}",
                    err
                )
            });
            let genesis_block = nodes[0].clone().genesis_block.unwrap();
            let lock_config = secp_lock_config_from_arguments(arguments);
            let users = {
                let owner_byte32_privkey =
                    Byte32::from_slice(H256::from_str(&owner_raw_privkey).unwrap().as_bytes())
                        .unwrap_or_else(|err| {
                            prompt_and_exit!(
                                "failed to parse CKB_BENCH_OWNER_PRIVKEY to Byte32, error: {}",
                                err
                            )
                        });
                let privkeys = derive_privkeys(owner_byte32_privkey, n_users);
                privkeys
                    .into_iter()
                    .map(|privkey| {
                        User::new_with_lock_config(
                            genesis_block.clone(),
                            Some(privkey),
                            lock_config.clone(),
                        )
                    })
                    .collect::<Vec<_>>()
            };
            crate::info!("info with params --n-users {}", users.len());
            let owner_byte32_privkey =
                Byte32::from_slice(H256::from_str(&owner_raw_privkey).unwrap().as_bytes())
                    .unwrap_or_else(|err| {
                        prompt_and_exit!(
                                "failed to parse CKB_BENCH_OWNER_PRIVKEY to Byte32, error: {}",
                                err
                            )
            });
            let owner_key = Privkey::from_slice(owner_byte32_privkey.as_slice());
            let owner = User::new_with_lock_config(
                genesis_block.clone(),
                Some(owner_key),
                lock_config,
            );
            let live_cells = owner.get_spendable_single_secp256k1_cells(&nodes[0]);
            let owner_capacity: u64 = live_cells.iter().map(|cell| cell.output.capacity.value()).sum();
            crate::info!("owner address:{},balance:{} live cells:{}", owner.single_secp256k1_address(), owner_capacity, live_cells.len());

            let mut total_capacity_sum: u128 = 0;
            for (i, user) in users.iter().enumerate() {
                let live_cells = user.get_spendable_single_secp256k1_cells(&nodes[0]);
                let user_capacity: u64 = live_cells.iter().map(|cell| cell.output.capacity.value()).sum();
                println!(
                    "user {} address:{} balance:{} live cells:{}",
                    i,
                    user.single_secp256k1_address(),
                    user_capacity,
                    live_cells.len()
                );
                total_capacity_sum += user_capacity as u128;
            }
            println!("total balance of {} users: {}", users.len(), total_capacity_sum);
        }
        ("miner", Some(arguments)) => {
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let n_blocks = value_t_or_exit!(arguments, "n-blocks", u64);
            let mining_interval_ms = value_t_or_exit!(arguments, "mining-interval-ms", u64);
            let min_tx_size = value_t_or_exit!(arguments, "min-tx-size", usize);
            let min_pending_tx_size =
                value_t_or_exit!(arguments, "min-pending-tx-size", u64);
            let nodes: Nodes = rpc_urls
                .iter()
                .map(|url| Node::init(url.as_str(), url.as_str()))
                .collect::<Vec<_>>()
                .into();

            // ensure nodes be out of ibd
            let max_tip_number = nodes
                .nodes()
                .map(|node| node.rpc_client().get_tip_block_number().unwrap())
                .max()
                .unwrap();
            if max_tip_number.value() == 0 {
                for node in nodes.nodes() {
                    node.mine(1, min_tx_size, min_pending_tx_size);
                    break;
                }
            }

            // connect nodes
            // nodes.p2p_connect();

            let max_tip_number = nodes
                .nodes()
                .map(|node| node.rpc_client().get_tip_block_number().unwrap())
                .max()
                .unwrap();
            while nodes
                .nodes()
                .any(|node| node.rpc_client().get_tip_block_number().unwrap() < max_tip_number)
            {
                sleep(Duration::from_secs(10));
                crate::info!("wait nodes sync");
            }

            // mine `n_blocks`
            let mut mined_n_blocks = 0;
            let mut highest_fixed_tip_number =
                nodes.get_fixed_header().inner.number.value();
            let mut last_print_instant = Instant::now();
            loop {
                for node in nodes.nodes() {
                    node.mine(1, min_tx_size, min_pending_tx_size);
                    let fixed_tip_number =
                        nodes.get_fixed_header().inner.number.value();
                    mined_n_blocks += count_new_fixed_tip_blocks(
                        &mut highest_fixed_tip_number,
                        fixed_tip_number,
                    );
                    if n_blocks != 0 && mined_n_blocks >= n_blocks {
                        return;
                    }

                    if last_print_instant.elapsed() >= Duration::from_secs(10) {
                        last_print_instant = Instant::now();
                        if n_blocks == 0 {
                            crate::info!(
                                "mined {} blocks, fixed_tip_number: {}",
                                mined_n_blocks,
                                fixed_tip_number
                            );
                        } else {
                            crate::info!(
                                "mined {}/{} blocks, fixed_tip_number: {}",
                                mined_n_blocks,
                                n_blocks,
                                fixed_tip_number
                            );
                        }
                    }
                    if mining_interval_ms != 0 {
                        sleep(Duration::from_millis(mining_interval_ms));
                    }
                }
            }
        }
        ("dispatch", Some(arguments)) => {
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let nodes = rpc_urls
                .iter()
                .map(|url| {
                    Node::init(url.as_str(), url.as_str())
                })
                .collect::<Vec<_>>();
            let n_users = value_t_or_exit!(arguments, "n-users", usize);
            let cells_per_user = value_t_or_exit!(arguments, "cells-per-user", u64);
            let capacity_per_cell = value_t_or_exit!(arguments, "capacity-per-cell", u64);
            let owner_raw_privkey = env::var("CKB_BENCH_OWNER_PRIVKEY").unwrap_or_else(|err| {
                prompt_and_exit!(
                    "cannot find \"CKB_BENCH_OWNER_PRIVKEY\" from environment variables, error: {}",
                    err
                )
            });
            let genesis_block = nodes[0].clone().genesis_block.unwrap();
            let lock_config = secp_lock_config_from_arguments(arguments);
            let owner = {
                let owner_privkey = Privkey::from_str(&owner_raw_privkey).unwrap_or_else(|err| {
                    prompt_and_exit!(
                        "failed to parse CKB_BENCH_OWNER_PRIVKEY to Privkey, error: {}",
                        err
                    )
                });
                User::new_with_lock_config(
                    genesis_block.clone(),
                    Some(owner_privkey),
                    lock_config.clone(),
                )
            };
            let users = {
                let owner_byte32_privkey =
                    Byte32::from_slice(H256::from_str(&owner_raw_privkey).unwrap().as_bytes())
                        .unwrap_or_else(|err| {
                            prompt_and_exit!(
                                "failed to parse CKB_BENCH_OWNER_PRIVKEY to Byte32, error: {}",
                                err
                            )
                        });
                let privkeys = derive_privkeys(owner_byte32_privkey, n_users);
                privkeys
                    .into_iter()
                    .map(|privkey| {
                        User::new_with_lock_config(
                            genesis_block.clone(),
                            Some(privkey),
                            lock_config.clone(),
                        )
                    })
                    .collect::<Vec<_>>()
            };
            dispatch(&nodes, &owner, &users, cells_per_user, capacity_per_cell);
        }
        ("collect", Some(arguments)) => {
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let nodes = rpc_urls
                .iter()
                .map(|url| {
                    Node::init(url.as_str(), url.as_str())
                })
                .collect::<Vec<_>>();
            let n_users = value_t_or_exit!(arguments, "n-users", usize);
            let owner_raw_privkey = env::var("CKB_BENCH_OWNER_PRIVKEY").unwrap_or_else(|err| {
                prompt_and_exit!(
                    "cannot find \"CKB_BENCH_OWNER_PRIVKEY\" from environment variables, error: {}",
                    err
                )
            });
            let genesis_block = nodes[0].clone().genesis_block.unwrap();
            let lock_config = secp_lock_config_from_arguments(arguments);
            let owner = {
                let owner_privkey = Privkey::from_str(&owner_raw_privkey).unwrap_or_else(|err| {
                    prompt_and_exit!(
                        "failed to parse CKB_BENCH_OWNER_PRIVKEY to Privkey, error: {}",
                        err
                    )
                });
                User::new_with_lock_config(
                    genesis_block.clone(),
                    Some(owner_privkey),
                    lock_config.clone(),
                )
            };
            let users = {
                let owner_byte32_privkey =
                    Byte32::from_slice(H256::from_str(&owner_raw_privkey).unwrap().as_bytes())
                        .unwrap_or_else(|err| {
                            prompt_and_exit!(
                                "failed to parse CKB_BENCH_OWNER_PRIVKEY to Byte32, error: {}",
                                err
                            )
                        });
                let privkeys = derive_privkeys(owner_byte32_privkey, n_users);
                privkeys
                    .into_iter()
                    .map(|privkey| {
                        User::new_with_lock_config(
                            genesis_block.clone(),
                            Some(privkey),
                            lock_config.clone(),
                        )
                    })
                    .collect::<Vec<_>>()
            };
            collect(&nodes, &owner, &users);
        }
        ("bench", Some(arguments)) => {
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let nodes = rpc_urls
                .iter()
                .map(|url| {
                    Node::init(url.as_str(), url.as_str())
                })
                .collect::<Vec<_>>();
            let n_users = value_t_or_exit!(arguments, "n-users", usize);
            let n_inout = value_t_or_exit!(arguments, "n-inout", usize);
            let t_tx_interval = {
                let tx_interval_ms = value_t_or_exit!(arguments, "tx-interval-ms", u64);
                Duration::from_millis(tx_interval_ms)
            };
            let t_bench = {
                let bench_time_ms = value_t_or_exit!(arguments, "bench-time-ms", u64);
                Duration::from_millis(bench_time_ms)
            };
            let owner_raw_privkey = env::var("CKB_BENCH_OWNER_PRIVKEY").unwrap_or_else(|err| {
                prompt_and_exit!(
                    "cannot find \"CKB_BENCH_OWNER_PRIVKEY\" from environment variables, error: {}",
                    err
                )
            });
            let genesis_block = nodes[0].clone().genesis_block.unwrap();
            let lock_config = secp_lock_config_from_arguments(arguments);
            let users = {
                let owner_byte32_privkey =
                    Byte32::from_slice(H256::from_str(&owner_raw_privkey).unwrap().as_bytes())
                        .unwrap_or_else(|err| {
                            prompt_and_exit!(
                                "failed to parse CKB_BENCH_OWNER_PRIVKEY to Byte32, error: {}",
                                err
                            )
                        });
                let privkeys = derive_privkeys(owner_byte32_privkey, n_users);
                privkeys
                    .into_iter()
                    .map(|privkey| {
                        User::new_with_lock_config(
                            genesis_block.clone(),
                            Some(privkey),
                            lock_config.clone(),
                        )
                    })
                    .collect::<Vec<_>>()
            };
            let add_tx_params_path = match value_t!(arguments, "add-tx-params", String) {
                Ok(path) => {
                    path
                }
                Err(_) => {
                    "".to_string()
                }
            };
            let add_tx_param = get_add_tx_param_by_path(add_tx_params_path);
            let is_smoking_test = arguments.is_present("is-smoking-test");
            let is_skip_report = arguments.is_present("is-skip-report");
            let bench_concurrent_requests_number = value_t_or_exit!(arguments, "concurrent-requests", usize);
            let (live_cell_sender, live_cell_receiver) = bounded(10000000);
            let (transaction_sender, transaction_receiver) = bounded(1000000);

            crate::info!(
                "bench with params --n-users {} --n-inout {} --tx-interval-ms {} --bench-time-ms {} --concurrent-requests {}",
                users.len(), n_inout, t_tx_interval.as_millis(), t_bench.as_millis(),bench_concurrent_requests_number
            );

            let live_cell_producer =
                LiveCellProducer::new(users.clone(), nodes.clone(), &add_tx_param);
            spawn(move || {
                live_cell_producer.run(live_cell_sender, 3);
            });


            let transaction_producer = TransactionProducer::new(
                users.clone(),
                vec![users[0].single_secp256k1_cell_dep()],
                n_inout,
                add_tx_param,
            );

            spawn(move || {
                transaction_producer.run(live_cell_receiver, transaction_sender, 3);
            });
            let watcher_status = Watcher::new(nodes.clone().into());

            let (pending_pool_sender, pending_pool_receiver) = bounded(1000000);
            spawn(move || {
                let ret = watcher_status.check_statue(3, t_bench,is_smoking_test);
                pending_pool_sender.send(ret).unwrap();
            });

            let (memory_usage_report_sender, memory_usage_report_receiver) = bounded(1000000);
            let (get_memory_usage_stop_sender, get_memory_usage_stop_receiver) = bounded(1000000);

            match value_t!(arguments, "prometheus-url", String) {
                Ok(url) => {
                    let client = MemoryUsageClient::new(url.clone());

                    crate::info!("start monit memory usage prometheus-url:{}",url);
                    spawn(move || {
                        let ret = client.get_memory_usage(Duration::from_secs(3).as_secs(), get_memory_usage_stop_receiver);
                        memory_usage_report_sender.send(ret).unwrap();
                    });
                }
                Err(_) => {
                    memory_usage_report_sender.send(MemoryUsageReport {
                        ckb_sys_mem_process_rss_mb: vec![],
                        ckb_sys_mem_process_vms_mb: vec![],
                        timestamp: vec![],
                    }).unwrap();
                }
            };

            let watcher = Watcher::new(nodes.clone().into());
            if !is_smoking_test {
                while !watcher.is_zero_load() {
                    sleep(Duration::from_secs(10));
                    crate::info!(
                        "[Watcher] is waiting the node become zero-load, fixed_tip_number: {}",
                        watcher.get_fixed_header().inner.number.value()
                    );
                }
            }

            let zero_load_number = watcher.get_fixed_header().inner.number;
            let rt = Runtime::new().unwrap();
            let tx_consumer = TransactionConsumer::new(nodes.clone());
            crate::info!("---- tx_consumer------");
            let mut set_tps = 0;
            let run_report_result = match value_t!(arguments, "tps", usize) {
                Ok(tps) => {
                    set_tps = tps;
                    rt.block_on(
                        tx_consumer.run_tps(transaction_receiver, bench_concurrent_requests_number, tps, t_bench)
                    )
                }
                Err(_) => {
                    rt.block_on(
                        tx_consumer.run(transaction_receiver, bench_concurrent_requests_number, t_tx_interval, t_bench)
                    )
                }
            };
            if is_skip_report {
                crate::info!("----finished-----");
                return;
            }
            if !is_smoking_test {
                while !watcher.is_zero_load() {
                    sleep(Duration::from_secs(10));
                    crate::info!(
                        "[Watcher] is waiting the node become zero-load, fixed_tip_number: {}",
                        watcher.get_fixed_header().inner.number.value()
                    );
                }
            }

            let pending_pool_report = pending_pool_receiver.recv().unwrap();
            get_memory_usage_stop_sender.send(true).unwrap();
            let memory_usage_report = memory_usage_report_receiver.recv().unwrap();

            let t_stat = t_bench.div(2);
            let fixed_tip_number = watcher.get_fixed_header().inner.number;
            let mut report = stat::stat(
                &nodes[0],
                (zero_load_number.value() + 1).into(),
                fixed_tip_number.into(),
                t_stat,
                Some(t_tx_interval),
            );
            let block_stat = stat::stat_metric(&nodes[0], (zero_load_number.value() + 1).into(),
                                               fixed_tip_number.into());

            report.set_send_tps = set_tps;
            report.client_send_tps = run_report_result.sum_tps;
            let html_data = generate_html_report(&TotalReport {
                block_report: block_stat.clone(),
                stat_report: report.clone(),
                pool_report: pending_pool_report.clone(),
                run_report: run_report_result.clone(),
                memory_usage_report: memory_usage_report.clone(),
            });
            let json_data = serde_json::to_string(&TotalReport {
                block_report: block_stat,
                stat_report: report.clone(),
                pool_report: pending_pool_report,
                run_report: run_report_result,
                memory_usage_report,
            }).unwrap();

            // Write JSON data to a file
            write_to_file("report.json", &json_data).expect("TODO: panic message");
            write_to_file("report.html", &html_data).expect("TODO: panic message");

            crate::info!(
                "markdown report: | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {}",
                report.ckb_version,
                report.transactions_per_second,
                report.n_inout,
                report.n_nodes,
                report.delay_time_ms.expect("bench specify delay_time_ms"),
                report.average_block_time_ms,
                report.average_block_transactions,
                report.average_block_transactions_size,
                report.from_block_number,
                report.to_block_number,
                report.total_transactions,
                report.total_transactions_size,
                report.transactions_size_per_second,
                report.set_send_tps,
                report.client_send_tps
            );
            crate::info!("metrics: {}", serde_json::json!(report));
        }
        ("watch", Some(arguments)) => {
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let internal_s1 = value_t_or_exit!(arguments, "interval-s", u64);
            let time_s = value_t_or_exit!(arguments, "time-s", u64);
            let nodes: Nodes = rpc_urls
                .iter()
                .map(|url| Node::init(url.as_str(), url.as_str()))
                .collect::<Vec<_>>()
                .into();
            let watch = Watcher::new(nodes);
            watch.check_statue(internal_s1, Duration::from_secs(time_s),false);
        }
        ("stat", Some(arguments)) => {
            let rpc_urls = values_t_or_exit!(arguments, "rpc-urls", Url);
            let from_number = value_t_or_exit!(arguments, "from-number", BlockNumber);
            let to_number = value_t_or_exit!(arguments, "to-number", BlockNumber);
            let stat_time_ms = value_t_or_exit!(arguments, "stat-period-ms", u64);
            let t_stat = Duration::from_millis(stat_time_ms);
            let node = Node::init(rpc_urls[0].as_str(), rpc_urls[0].as_str());
            let report = stat::stat(&node, from_number, to_number, t_stat, None);
            crate::info!("metrics: {}", serde_json::json!(report));
        }
        _ => {
            eprintln!("wrong usage");
            exit(1);
        }
    }
}

fn parse_h256_argument(raw: &str) -> Result<H256, String> {
    H256::from_str(raw.strip_prefix("0x").unwrap_or(raw)).map_err(|err| err.to_string())
}

fn count_new_fixed_tip_blocks(highest_fixed_tip_number: &mut u64, fixed_tip_number: u64) -> u64 {
    if fixed_tip_number <= *highest_fixed_tip_number {
        return 0;
    }
    let new_blocks = fixed_tip_number - *highest_fixed_tip_number;
    *highest_fixed_tip_number = fixed_tip_number;
    new_blocks
}

fn network_type_from_argument(raw: &str) -> NetworkType {
    match raw {
        "mainnet" => NetworkType::Mainnet,
        "testnet" => NetworkType::Testnet,
        "dev" => NetworkType::Dev,
        "staging" => NetworkType::Staging,
        _ => unreachable!("validated network"),
    }
}

fn owner_privkey_from_env() -> Privkey {
    let raw_privkey = env::var("CKB_BENCH_OWNER_PRIVKEY").unwrap_or_else(|err| {
        prompt_and_exit!(
            "cannot find \"CKB_BENCH_OWNER_PRIVKEY\" from environment variables, error: {}",
            err
        )
    });
    Privkey::from_str(&raw_privkey).unwrap_or_else(|err| {
        prompt_and_exit!(
            "failed to parse CKB_BENCH_OWNER_PRIVKEY as a private key, error: {}",
            err
        )
    })
}

fn address_from_privkey(
    privkey: &Privkey,
    network: NetworkType,
    lock_config: Option<&SecpLockConfig>,
) -> Address {
    let pubkey = privkey.pubkey().expect("validated owner private key");
    let args = ckb_types::H160::from_slice(&blake2b_256(pubkey.serialize())[0..20])
        .expect("Blake160 is always 20 bytes");
    let payload = match lock_config {
        Some(config) => AddressPayload::from(config.lock_script(args)),
        None => AddressPayload::from_pubkey_hash(args),
    };
    Address::new(network, payload, true)
}

fn secp_lock_config_from_arguments(arguments: &ArgMatches) -> Option<SecpLockConfig> {
    let code_hash = arguments.value_of("lock-code-hash")?;
    let code_hash = parse_h256_argument(code_hash).expect("validated lock code hash");
    let hash_type = match arguments.value_of("lock-hash-type").unwrap_or("type") {
        "type" => ScriptHashType::Type,
        "data" => ScriptHashType::Data,
        "data1" => ScriptHashType::Data1,
        "data2" => ScriptHashType::Data2,
        _ => unreachable!("validated lock hash type"),
    };
    let cell_dep = arguments.value_of("lock-dep-tx-hash").map(|tx_hash| {
        let tx_hash =
            parse_h256_argument(tx_hash).expect("validated lock dep transaction hash");
        let index = arguments
            .value_of("lock-dep-index")
            .unwrap_or("0")
            .parse::<u32>()
            .expect("validated lock dep index");
        let dep_type = match arguments.value_of("lock-dep-type").unwrap_or("code") {
            "code" => DepType::Code,
            "dep-group" => DepType::DepGroup,
            _ => unreachable!("validated lock dep type"),
        };
        CellDep::new_builder()
            .out_point(OutPoint::new(tx_hash.pack(), index))
            .dep_type(dep_type.into())
            .build()
    });

    Some(SecpLockConfig::new(code_hash, hash_type, cell_dep))
}

fn lock_script_args() -> Vec<Arg<'static, 'static>> {
    vec![
        Arg::with_name("lock-code-hash")
            .long("lock-code-hash")
            .takes_value(true)
            .value_name("HASH")
            .help("Override the secp-compatible lock Script code_hash")
            .validator(|s| parse_h256_argument(&s).map(|_| ())),
        Arg::with_name("lock-hash-type")
            .long("lock-hash-type")
            .takes_value(true)
            .value_name("TYPE")
            .possible_values(&["type", "data", "data1", "data2"])
            .requires("lock-code-hash")
            .help("hash_type for --lock-code-hash; defaults to type"),
        Arg::with_name("lock-dep-tx-hash")
            .long("lock-dep-tx-hash")
            .takes_value(true)
            .value_name("HASH")
            .requires("lock-code-hash")
            .help("Transaction hash containing the custom lock dependency")
            .validator(|s| parse_h256_argument(&s).map(|_| ())),
        Arg::with_name("lock-dep-index")
            .long("lock-dep-index")
            .takes_value(true)
            .value_name("INDEX")
            .requires("lock-dep-tx-hash")
            .help("Output index of the custom lock dependency; defaults to 0")
            .validator(|s| s.parse::<u32>().map(|_| ()).map_err(|err| err.to_string())),
        Arg::with_name("lock-dep-type")
            .long("lock-dep-type")
            .takes_value(true)
            .value_name("TYPE")
            .possible_values(&["code", "dep-group"])
            .requires("lock-dep-tx-hash")
            .help("Dependency type for the custom lock; defaults to code"),
    ]
}

fn clap_app() -> App<'static, 'static> {
    include_str!("../Cargo.toml");
    App::new("ckb-bench")
        .version(git_version::git_version!())
        .subcommand(
            SubCommand::with_name("address")
                .visible_alias("get-address")
                .about("print the funding address for CKB_BENCH_OWNER_PRIVKEY")
                .arg(
                    Arg::with_name("network")
                        .long("network")
                        .value_name("NETWORK")
                        .takes_value(true)
                        .possible_values(&["mainnet", "testnet", "dev", "staging"])
                        .default_value("testnet")
                        .help("Network prefix for the address"),
                )
                .args(&lock_script_args()),
        )
        .subcommand(
            SubCommand::with_name("info")
                .about("query balances of N users")
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .value_name("URLS")
                        .help("CKB rpc urls, prefix with network protocol, delimited by comma, e.g. \"http://127.0.0.1:8114,http://127.0.0.2.8114\"")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .use_delimiter(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("n-users")
                        .long("n-users")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("Number of users")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .args(&lock_script_args()),
        )
        .subcommand(
            SubCommand::with_name("miner")
                .about("runs ckb miner")
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .value_name("URLS")
                        .long_help("CKB rpc urls, prefix with network protocol, delimited by comma, e.g. \"http://127.0.0.1:8114,http://127.0.0.2.8114\"")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .use_delimiter(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("n-blocks")
                        .short("b")
                        .long("n-blocks")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("How many blocks to mine, 0 means infinitely")
                        .default_value("0")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("mining-interval-ms")
                        .long("mining-interval-ms")
                        .value_name("TIME")
                        .takes_value(true)
                        .help("How long it takes to mine a block.\nNote that it is different with \"block time interval\", we can/should not control the block time interval")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                ).arg(
                Arg::with_name("min-tx-size")
                    .short("t")
                    .long("min-tx-size")
                    .value_name("NUMBER")
                    .takes_value(true)
                    .help("How min tx to mine, 0 means empty block could miner")
                    .default_value("0")
                    .required(true)
                    .validator(|s| s.parse::<usize>().map(|_| ()).map_err(|err| err.to_string())),
            ).arg(
                Arg::with_name("min-pending-tx-size")
                    .long("min-pending-tx-size")
                    .value_name("NUMBER")
                    .takes_value(true)
                    .help("Minimum pending transaction count required to mine a block")
                    .default_value("0")
                    .required(true)
                    .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
            ),
        )
        .subcommand(
            SubCommand::with_name("bench")
                .about("bench the target ckb nodes")
                .arg(
                    Arg::with_name("add-tx-params")
                        .long("add-tx-params")
                        .required(false)
                        .takes_value(true)
                        .value_name("PATH")
                        .help("add tx  params"),
                )
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .value_name("URLS")
                        .help("CKB rpc urls, prefix with network protocol, delimited by comma, e.g. \"http://127.0.0.1:8114,http://127.0.0.2.8114\"")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .use_delimiter(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("n-users")
                        .long("n-users")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("Number of users")
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("n-inout")
                        .long("n-inout")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("input-output pairs of a transaction")
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("tx-interval-ms")
                        .long("tx-interval-ms")
                        .value_name("TIME")
                        .takes_value(true)
                        .help("Interval of sending transactions in milliseconds")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("bench-time-ms")
                        .long("bench-time-ms")
                        .value_name("TIME")
                        .takes_value(true)
                        .help("Bench time period")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("is-smoking-test")
                        .long("is-smoking-test")
                        .help("Whether the target network is production network, like mainnet, testnet, devnet"),
                )
                .arg(
                    Arg::with_name("is-skip-report")
                        .long("is-skip-report")
                        .help("skip collect report"),
                )
                .arg(
                    Arg::with_name("concurrent-requests")
                        .long("concurrent-requests")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .default_value("1")
                        .help("Bench concurrent requests")
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("tps")
                        .long("tps")
                        .value_name("NUMBER")
                        .required(false)
                        .help("Set the fixed load for transactions per second (TPS)")
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                ).arg(
                Arg::with_name("prometheus-url")
                    .long("prometheus-url")
                    .value_name("URL")
                    .required(false)
                    .help("node prometheus url")
            )
                .args(&lock_script_args())
            ,
        )
        .subcommand(
            SubCommand::with_name("dispatch")
                .about("dispatch capacity to users")
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .value_name("URLS")
                        .help("CKB rpc urls, prefix with network protocol, delimited by comma, e.g. \"http://127.0.0.1:8114,http://127.0.0.2.8114\"")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .use_delimiter(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("n-users")
                        .long("n-users")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("Number of users")
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("cells-per-user")
                        .long("cells-per-user")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("Cells per user")
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("capacity-per-cell")
                        .long("capacity-per-cell")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .required(true)
                        .help("Capacity per cell")
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("data-dir")
                        .long("data-dir")
                        .required(true)
                        .takes_value(true)
                        .value_name("PATH")
                        .default_value("./data")
                        .help("Data directory"),
                )
                .args(&lock_script_args())
        )
        .subcommand(
            SubCommand::with_name("collect")
                .about("collect capacity back to owner")
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .value_name("URLS")
                        .help("CKB rpc urls, prefix with network protocol, delimited by comma, e.g. \"http://127.0.0.1:8114,http://127.0.0.2.8114\"")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .use_delimiter(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("n-users")
                        .long("n-users")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("Number of users")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("data-dir")
                        .long("data-dir")
                        .required(true)
                        .takes_value(true)
                        .value_name("PATH")
                        .default_value("./data")
                        .help("Data directory"),
                )
                .args(&lock_script_args()),
        ).subcommand(
        SubCommand::with_name("watch")
            .about("watch chain stat")
            .arg(
                Arg::with_name("rpc-urls")
                    .long("rpc-urls")
                    .value_name("URLS")
                    .long_help("CKB rpc urls, prefix with network protocol, delimited by comma, e.g. \"http://127.0.0.1:8114,http://127.0.0.2.8114\"")
                    .required(true)
                    .takes_value(true)
                    .multiple(true)
                    .use_delimiter(true)
                    .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
            ).arg(
            Arg::with_name("interval-s")
                .long("interval-s")
                .value_name("NUMBER")
                .takes_value(false)
                .default_value("3")
                .help("interval time")
                .required(true)
                .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
        ).arg(
            Arg::with_name("time-s")
                .long("time-s")
                .value_name("NUMBER")
                .takes_value(false)
                .default_value("600")
                .help("long time")
                .required(true)
                .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
        )
    )
        .subcommand(
            SubCommand::with_name("stat")
                .about("report chain stat")
                .arg(
                    Arg::with_name("rpc-urls")
                        .long("rpc-urls")
                        .value_name("URLS")
                        .long_help("CKB rpc urls, prefix with network protocol, delimited by comma, e.g. \"http://127.0.0.1:8114,http://127.0.0.2.8114\"")
                        .required(true)
                        .takes_value(true)
                        .multiple(true)
                        .use_delimiter(true)
                        .validator(|s| Url::parse(&s).map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("from-number")
                        .long("from-number")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("From block number")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("to-number")
                        .long("to-number")
                        .value_name("NUMBER")
                        .takes_value(true)
                        .help("To block number")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                )
                .arg(
                    Arg::with_name("stat-period-ms")
                        .long("stat-period-ms")
                        .value_name("TIME")
                        .takes_value(true)
                        .help("Stat period")
                        .required(true)
                        .validator(|s| s.parse::<u64>().map(|_| ()).map_err(|err| err.to_string())),
                ),
        )
}

fn init_logger() -> ckb_logger_service::LoggerInitGuard {
    let filter = match env::var("RUST_LOG") {
        Ok(filter) if filter.is_empty() => Some("info".to_string()),
        Ok(filter) => Some(filter.to_string()),
        Err(_) => Some("info".to_string()),
    };
    let config = ckb_logger_config::Config {
        filter,
        color: false,
        log_to_file: false,
        log_to_stdout: true,
        ..Default::default()
    };
    ckb_logger_service::init(None, config)
        .unwrap_or_else(|err| panic!("failed to init the logger service, error: {}", err))
}

//
//
// mod logger;
// mod  node;
// mod nodes;
// pub mod util;
// mod watcher;
// mod utils;
// mod user;
// mod stat;
// mod prepare;
// mod bench;
//
// fn main() {
//     let _logger = init_logger();
//     // use ckb_sdk::rpc::CkbRpcClient;
//     //
//     // let mut ckb_client = CkbRpcClient::new("https://testnet.ckb.dev");
//     // let block = ckb_client.get_block_by_number(0.into()).unwrap();
//     // println!("block: {}", serde_json::to_string_pretty(&block).unwrap());
//     let mut node = Node::init("https://testnet.ckb.dev", "https://testnet.ckb.dev");
//     let block = node.rpc_client.get_block_by_number(0.into()).unwrap();
//     // println!("block: {}", serde_json::to_string_pretty(&block).unwrap());
//
//
// }
//
//
// fn init_logger() -> ckb_logger_service::LoggerInitGuard {
//     let filter = match env::var("RUST_LOG") {
//         Ok(filter) if filter.is_empty() => Some("info".to_string()),
//         Ok(filter) => Some(filter.to_string()),
//         Err(_) => Some("info".to_string()),
//     };
//     let config = ckb_logger_config::Config {
//         filter,
//         color: false,
//         log_to_file: false,
//         log_to_stdout: true,
//         ..Default::default()
//     };
//     ckb_logger_service::init(None, config)
//         .unwrap_or_else(|err| panic!("failed to init the logger service, error: {}", err))
// }


// fn main() {
//
//     let args: Vec<String> = env::args().collect();
//
//     if args.len() < 2 {
//         panic!("Please provide the file path as an argument");
//     }
//
//     let dd = get_add_tx_param_by_path(file_path);
//
//     // let mut scpt: ScriptOpt = {
//     //     if deserialized_object._type == ckb_jsonrpc_types::Script::default(){
//     //         // return Some(ckb_types::packed::Script::from(deserialized_object._type.into()).clone())
//     //
//     //     }
//     //     None
//     // };
// }

fn get_add_tx_param_by_path(file_path: String) -> AddTxParam {
    if file_path == "" {
        return AddTxParam::new();
    }
    let mut file = File::open(file_path).expect("not found file path");
    let mut json_content = String::new();
    file.read_to_string(&mut json_content).expect("Failed to read the file");
    let deserialized_object: AddTxParam = serde_json::from_str(&json_content).expect("Failed to deserialize JSON");
    deserialized_object
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    const CUSTOM_CODE_HASH: &str =
        "0x1111111111111111111111111111111111111111111111111111111111111111";
    const CUSTOM_DEP_TX_HASH: &str =
        "0x2222222222222222222222222222222222222222222222222222222222222222";

    #[test]
    fn info_accepts_custom_lock_arguments() {
        let matches = clap_app()
            .get_matches_from_safe(vec![
                "ckb-bench",
                "info",
                "--rpc-urls",
                "http://127.0.0.1:8114",
                "--n-users",
                "1",
                "--lock-code-hash",
                CUSTOM_CODE_HASH,
                "--lock-hash-type",
                "data1",
                "--lock-dep-tx-hash",
                CUSTOM_DEP_TX_HASH,
                "--lock-dep-index",
                "3",
                "--lock-dep-type",
                "dep-group",
            ])
            .unwrap();
        let arguments = matches.subcommand_matches("info").unwrap();

        assert!(secp_lock_config_from_arguments(arguments).is_some());
    }

    #[test]
    fn lock_hash_type_requires_code_hash() {
        let result = clap_app().get_matches_from_safe(vec![
            "ckb-bench",
            "info",
            "--rpc-urls",
            "http://127.0.0.1:8114",
            "--n-users",
            "1",
            "--lock-hash-type",
            "data1",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn default_arguments_keep_standard_lock_configuration() {
        let matches = clap_app()
            .get_matches_from_safe(vec![
                "ckb-bench",
                "info",
                "--rpc-urls",
                "http://127.0.0.1:8114",
                "--n-users",
                "1",
            ])
            .unwrap();
        let arguments = matches.subcommand_matches("info").unwrap();

        assert!(secp_lock_config_from_arguments(arguments).is_none());
    }

    #[test]
    fn miner_accepts_min_pending_tx_size() {
        let matches = clap_app()
            .get_matches_from_safe(vec![
                "ckb-bench",
                "miner",
                "--rpc-urls",
                "http://127.0.0.1:8114",
                "--mining-interval-ms",
                "100",
                "--min-pending-tx-size",
                "1400",
            ])
            .unwrap();
        let arguments = matches.subcommand_matches("miner").unwrap();

        assert_eq!(arguments.value_of("min-pending-tx-size"), Some("1400"));
    }

    #[test]
    fn miner_min_pending_tx_size_defaults_to_zero() {
        let matches = clap_app()
            .get_matches_from_safe(vec![
                "ckb-bench",
                "miner",
                "--rpc-urls",
                "http://127.0.0.1:8114",
                "--mining-interval-ms",
                "100",
            ])
            .unwrap();
        let arguments = matches.subcommand_matches("miner").unwrap();

        assert_eq!(arguments.value_of("min-pending-tx-size"), Some("0"));
    }

    #[test]
    fn mined_count_only_tracks_new_fixed_tip_blocks() {
        let mut highest_fixed_tip_number = 100;

        assert_eq!(
            count_new_fixed_tip_blocks(&mut highest_fixed_tip_number, 100),
            0
        );
        assert_eq!(
            count_new_fixed_tip_blocks(&mut highest_fixed_tip_number, 99),
            0
        );
        assert_eq!(
            count_new_fixed_tip_blocks(&mut highest_fixed_tip_number, 103),
            3
        );
        assert_eq!(highest_fixed_tip_number, 103);
    }

    #[test]
    fn address_command_defaults_to_testnet_and_supports_alias() {
        for command in &["address", "get-address"] {
            let matches = clap_app()
                .get_matches_from_safe(vec!["ckb-bench", command])
                .unwrap();
            let arguments = matches.subcommand_matches("address").unwrap();

            assert_eq!(arguments.value_of("network"), Some("testnet"));
            assert_eq!(
                network_type_from_argument(arguments.value_of("network").unwrap()),
                NetworkType::Testnet
            );
        }
    }

    #[test]
    fn generated_address_matches_owner_lock_script() {
        let privkey = Privkey::from_str(
            "af44a4755acccdd932561db5163d5c2ac025faa00877719c78bb0b5d61da8c94",
        )
        .unwrap();

        let address = address_from_privkey(&privkey, NetworkType::Testnet, None);
        let expected_args =
            ckb_types::H160::from_slice(&blake2b_256(privkey.pubkey().unwrap().serialize())[..20])
                .unwrap();
        let expected_script = ckb_types::packed::Script::new_builder()
            .hash_type(ScriptHashType::Type.into())
            .code_hash(ckb_bench::SIGHASH_ALL_TYPE_HASH.pack())
            .args(expected_args.0.pack())
            .build();

        assert!(address.to_string().starts_with("ckt1"));
        assert_eq!(ckb_types::packed::Script::from(&address), expected_script);
    }

    #[test]
    fn generated_address_uses_custom_lock_script() {
        let privkey = Privkey::from_str(
            "af44a4755acccdd932561db5163d5c2ac025faa00877719c78bb0b5d61da8c94",
        )
        .unwrap();
        let config = SecpLockConfig::new(
            parse_h256_argument(CUSTOM_CODE_HASH).unwrap(),
            ScriptHashType::Data1,
            None,
        );

        let address = address_from_privkey(&privkey, NetworkType::Mainnet, Some(&config));
        let expected_args =
            ckb_types::H160::from_slice(&blake2b_256(privkey.pubkey().unwrap().serialize())[..20])
                .unwrap();

        assert!(address.to_string().starts_with("ckb1"));
        assert_eq!(
            ckb_types::packed::Script::from(&address),
            config.lock_script(expected_args)
        );
    }
}
