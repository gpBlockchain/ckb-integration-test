// use ckb_jsonrpc_types::BlockView;
// node is  rpc implement
use ckb_sdk::rpc::CkbRpcClient;
use std::time::{Duration, Instant};
use std::thread::sleep;
use ckb_jsonrpc_types::{BlockNumber, BlockView, JsonBytes, Script as JsonScript};
use ckb_sdk::rpc::ckb_indexer::{
    Cell, Order, SearchKey, SearchKeyFilter, SearchMode, ScriptType,
};
use ckb_sdk::RpcError;
use ckb_types::packed;
use ckb_types::packed::Script;
use ckb_types::prelude::{Builder, Entity};
use lazy_static::lazy_static;

use std::collections::HashMap;

/// The indexer rejects requests whose limit is greater than or equal to 1200.
pub const MAX_QUERY_CELL_SIZE: u32 = 1000;

use std::sync::{Arc, Mutex};
use crate::rpc::RpcClient;

lazy_static! {
    static ref MAP: Mutex<HashMap<String, Arc<CkbRpcClient>>> = Mutex::new(HashMap::new());
}

fn has_enough_pending_transactions(pending_tx_size: u64, min_pending_tx_size: u64) -> bool {
    pending_tx_size >= min_pending_tx_size
}

fn pure_capacity_filter() -> SearchKeyFilter {
    SearchKeyFilter {
        // Indexer ranges use an inclusive lower bound and exclusive upper bound.
        // A zero-length secondary script means the cell has no Type Script.
        script_len_range: Some([0u64.into(), 1u64.into()]),
        // Capacity-provider cells should not carry application data either.
        output_data_len_range: Some([0u64.into(), 1u64.into()]),
        ..Default::default()
    }
}

fn is_benchmark_cell_output(
    actual_type: Option<&JsonScript>,
    actual_data: Option<&JsonBytes>,
    target_type: Option<&JsonScript>,
    target_data: &JsonBytes,
) -> bool {
    let actual_data = actual_data.map(JsonBytes::as_bytes).unwrap_or_default();
    let is_pure_capacity = actual_type.is_none() && actual_data.is_empty();
    let is_target_contract = actual_type == target_type && actual_data == target_data.as_bytes();

    is_pure_capacity || is_target_contract
}


pub fn get_or_create_ckb_client(key: String) -> Arc<CkbRpcClient> {
    {
        let map = MAP.lock().unwrap();

        if let Some(value) = map.get(&key) {
            return Arc::clone(value);
        }
    }
    let mut map = MAP.lock().unwrap();
    let default_value = Arc::new(CkbRpcClient::new(key.as_str()));
    let value = map.entry(key).or_insert_with(|| Arc::clone(&default_value));
    Arc::clone(value)
}


#[derive(Debug, Clone, Default)]
pub struct NodeOptions {
    pub node_name: String,
}

pub struct Node {
    pub(super) rpc_client: String,
    //todo Remove async_client : because rust sdk not have async rpc client , blocking
    pub(super) async_client: RpcClient,
    pub(super) indexer: String,
    pub(super) genesis_block: Option<BlockView>,
    // initialize when node start
    pub(super) node_options: NodeOptions,
}

impl Node {
    pub fn init(ckb_rpc_url: &str, ckb_indexer_rpc_rul: &str) -> Self {
        get_or_create_ckb_client(ckb_indexer_rpc_rul.to_string());
        let ckb_client = get_or_create_ckb_client(ckb_rpc_url.to_string());
        let genesis_block = ckb_client.get_block_by_number(0.into()).unwrap();
        let client = RpcClient::new(&ckb_rpc_url);

        let mut node_opt = NodeOptions::default();
        node_opt.node_name = ckb_rpc_url.to_string();

        Self {
            rpc_client: ckb_rpc_url.to_string(),
            indexer: ckb_indexer_rpc_rul.to_string(),
            async_client: client,
            genesis_block,
            node_options: node_opt,
        }
    }
    pub fn node_name(&self) -> &str {
        &self.node_options.node_name
    }
    pub fn rpc_client(&self) -> Arc<CkbRpcClient> {
        get_or_create_ckb_client(self.rpc_client.to_string())
    }

    pub fn async_client(&self) -> &RpcClient {
        &self.async_client
    }

    pub fn get_tip_block(&self) -> BlockView {
        let rpc_client = self.rpc_client();
        let tip_number = rpc_client.get_tip_block_number().unwrap();
        let block = rpc_client
            .get_block_by_number(tip_number)
            .expect("tip block exists");
        crate::trace!(
            "[Node {}] Node::get_tip_block(), block: {:?}",
            self.node_name(),
            block
        );
        block.unwrap()
    }

    pub fn wait_for_tx_pool(&self) {
        let rpc_client = self.rpc_client();
        let mut chain_tip = rpc_client.get_tip_header().unwrap();
        let mut tx_pool_tip = rpc_client.tx_pool_info().unwrap();
        if chain_tip.hash == tx_pool_tip.tip_hash {
            return;
        }
        let mut instant = Instant::now();
        while instant.elapsed() < Duration::from_secs(10) {
            sleep(std::time::Duration::from_secs(1));
            chain_tip = rpc_client.get_tip_header().unwrap();
            let prev_tx_pool_tip = tx_pool_tip;
            tx_pool_tip = rpc_client.tx_pool_info().unwrap();
            if chain_tip.hash == tx_pool_tip.tip_hash {
                return;
            } else if prev_tx_pool_tip.tip_hash != tx_pool_tip.tip_hash
                && tx_pool_tip.tip_number.value() < chain_tip.inner.number.value()
            {
                instant = Instant::now();
            }
        }

        panic!(
            "timeout to wait for tx pool,\n\tchain   tip: {:?}, {:#x},\n\ttx-pool tip: {}, {:#x}",
            chain_tip.inner.number.value(),
            chain_tip.hash,
            tx_pool_tip.tip_number.value(),
            tx_pool_tip.tip_hash,
        );
    }
    pub fn indexer(&self) -> Arc<CkbRpcClient> {
        get_or_create_ckb_client(self.indexer.to_string())
    }

    pub fn get_pure_capacity_cells_by_lock_script(
        &self,
        script: Script,
    ) -> Result<Vec<Cell>, RpcError> {
        self.get_cells_by_lock_script(script, Some(pure_capacity_filter()), Some(false))
    }

    pub fn get_benchmark_cells_by_lock_script(
        &self,
        script: Script,
        target_type: Option<JsonScript>,
        target_data: JsonBytes,
    ) -> Result<Vec<Cell>, RpcError> {
        let mut cells = self.get_cells_by_lock_script(script, None, Some(true))?;
        cells.retain(|cell| {
            is_benchmark_cell_output(
                cell.output.type_.as_ref(),
                cell.output_data.as_ref(),
                target_type.as_ref(),
                &target_data,
            )
        });
        Ok(cells)
    }

    fn get_cells_by_lock_script(
        &self,
        script: Script,
        filter: Option<SearchKeyFilter>,
        with_data: Option<bool>,
    ) -> Result<Vec<Cell>, RpcError> {
        let search_key = SearchKey {
            script: Script::new_builder()
                .code_hash(script.code_hash())
                .hash_type(script.hash_type())
                .args(script.args())
                .build()
                .into(),
            script_type: ScriptType::Lock,
            script_search_mode: Some(SearchMode::Exact),
            filter,
            with_data,
            group_by_transaction: None,
        };

        let indexer = self.indexer();
        let mut cells = Vec::new();
        let mut cursor = None;

        loop {
            let page = indexer.get_cells(
                search_key.clone(),
                Order::Asc,
                MAX_QUERY_CELL_SIZE.into(),
                cursor,
            )?;
            let is_last_page = page.objects.len() < MAX_QUERY_CELL_SIZE as usize;
            cells.extend(page.objects);

            if is_last_page {
                return Ok(cells);
            }

            cursor = Some(page.last_cursor);
        }
    }

    pub fn mine(
        &self,
        n_blocks: u64,
        min_tx_size: usize,
        min_pending_tx_size: u64,
    ) {
        for _ in 0..n_blocks {
            let rpc_client = self.rpc_client();
            let pending_tx_size = rpc_client.tx_pool_info().unwrap().pending.value();
            if !has_enough_pending_transactions(pending_tx_size, min_pending_tx_size) {
                continue;
            }
            let template = rpc_client.get_block_template(None, None, None).unwrap();
            let block = packed::Block::from(template);
            if block.transactions().len() < min_tx_size && block.proposals().len() < min_tx_size {
                continue;
            }
            rpc_client.submit_block("".into(), block.into()).unwrap();
            self.wait_for_tx_pool();
        }
    }

    pub fn mine_to(&self, target_height: BlockNumber) {
        let tip_number = self.rpc_client().get_tip_block_number().unwrap();
        if tip_number.value() < target_height.value() {
            let n_blocks = target_height.value() - tip_number.value();
            self.mine(n_blocks.into(), 0, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        has_enough_pending_transactions, is_benchmark_cell_output, pure_capacity_filter,
        MAX_QUERY_CELL_SIZE,
    };
    use ckb_jsonrpc_types::{JsonBytes, Script};

    #[test]
    fn benchmark_output_selection_keeps_pure_and_exact_contract_cells() {
        let target_type = Script::default();
        let target_data = JsonBytes::from_vec(vec![1, 2, 3]);
        let other_type = Script {
            args: JsonBytes::from_vec(vec![9]),
            ..Default::default()
        };
        let empty_data = JsonBytes::default();
        let other_data = JsonBytes::from_vec(vec![4, 5, 6]);

        assert!(is_benchmark_cell_output(
            None,
            Some(&empty_data),
            Some(&target_type),
            &target_data,
        ));
        assert!(is_benchmark_cell_output(
            Some(&target_type),
            Some(&target_data),
            Some(&target_type),
            &target_data,
        ));
        assert!(!is_benchmark_cell_output(
            Some(&other_type),
            Some(&target_data),
            Some(&target_type),
            &target_data,
        ));
        assert!(!is_benchmark_cell_output(
            Some(&target_type),
            Some(&other_data),
            Some(&target_type),
            &target_data,
        ));
    }

    #[test]
    fn pure_capacity_filter_requires_no_type_script_or_data() {
        let filter = pure_capacity_filter();
        let script_len_range = filter.script_len_range.expect("script length range");
        let data_len_range = filter
            .output_data_len_range
            .expect("output data length range");

        assert_eq!(script_len_range[0].value(), 0);
        assert_eq!(script_len_range[1].value(), 1);
        assert_eq!(data_len_range[0].value(), 0);
        assert_eq!(data_len_range[1].value(), 1);
    }

    #[test]
    fn indexer_page_size_is_below_server_limit() {
        assert!(MAX_QUERY_CELL_SIZE < 1200);
    }

    #[test]
    fn pending_threshold_is_inclusive() {
        assert!(!has_enough_pending_transactions(1399, 1400));
        assert!(has_enough_pending_transactions(1400, 1400));
        assert!(has_enough_pending_transactions(1401, 1400));
    }

    #[test]
    fn zero_pending_threshold_is_disabled() {
        assert!(has_enough_pending_transactions(0, 0));
    }
}

impl Clone for Node {
    fn clone(&self) -> Node {
        Self {
            node_options: self.node_options.clone(),
            rpc_client: self.rpc_client.to_string(),
            async_client: self.async_client.clone(),
            genesis_block: self.genesis_block.clone(),
            indexer: self.indexer.to_string(),
        }
    }
}
