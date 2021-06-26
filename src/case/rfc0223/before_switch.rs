// Before RFC0223, since in absolute_epoch format can be
//   - absolute_epoch.index > absolute_epoch.length
//   - absolute_epoch.index = absolute_epoch.length > 0
//
// After RFC0223, since in absolute_epoch format must be
//   - absolute_epoch.index < absolute_epoch.length
//   - or absolute_epoch.index == absolute_epoch.length == 0
//
// Before RFC0223, since in relative_epoch format can be
//   - relative_epoch.index > relative_epoch.length
//   - relative_epoch.index = relative_epoch.length > 0
//
// After RFC0223, since in relative_epoch format must be
//   - relative_epoch.index < relative_epoch.length
//   - or relative_epoch.index == relative_epoch.length == 0

use crate::case::{Case, CaseOptions};
use crate::node::{Node, NodeOptions};
use crate::nodes::Nodes;
use crate::util::{
    since_from_absolute_epoch_number_with_fraction, since_from_relative_epoch_number_with_fraction,
};
use crate::{CKB_FORK0_BINARY, CKB_FORK2021_BINARY};
use ckb_types::{
    core::{cell::CellMeta, EpochNumber, EpochNumberWithFraction, TransactionBuilder},
    packed::{CellInput, CellOutput},
    prelude::*,
};

const RFC0223_EPOCH_NUMBER: EpochNumber = 3;

pub struct RFC0223BeforeSwitch;

impl Case for RFC0223BeforeSwitch {
    fn case_options(&self) -> CaseOptions {
        CaseOptions {
            make_all_nodes_connected: false,
            make_all_nodes_synced: false,
            make_all_nodes_connected_and_synced: false,
            node_options: vec![
                NodeOptions {
                    node_name: "node-fork0",
                    ckb_binary: CKB_FORK0_BINARY.lock().clone(),
                    initial_database: "db/empty",
                    chain_spec: "spec/fork2021",
                    app_config: "config/fork2021",
                },
                NodeOptions {
                    node_name: "node-fork2021",
                    ckb_binary: CKB_FORK2021_BINARY.lock().clone(),
                    initial_database: "db/empty",
                    chain_spec: "spec/fork2021",
                    app_config: "config/fork2021",
                },
            ]
            .into_iter()
            .collect(),
        }
    }

    fn run(&self, nodes: Nodes) {
        let node_fork0 = nodes.get_node("node-fork0");
        let node_fork2021 = nodes.get_node("node-fork2021");
        node_fork0.mine(1);
        node_fork2021.mine(1);
        nodes.p2p_connect();
        node_fork2021.mine(
            node_fork2021
                .consensus()
                .tx_proposal_window
                .farthest
                .value()
                + 4,
        );
        nodes.waiting_for_sync().expect("nodes should be synced");

        let current_block_epoch = node_fork2021.get_tip_block().epoch();
        let cells = node_fork0.get_live_always_success_cells();
        assert!(cells.len() >= 4);

        let build_transaction = |since: u64, input: &CellMeta| {
            TransactionBuilder::default()
                .input(CellInput::new(input.out_point.clone(), since))
                .output(
                    CellOutput::new_builder()
                        .lock(input.cell_output.lock())
                        .type_(input.cell_output.type_())
                        .capacity(input.capacity().pack())
                        .build(),
                )
                .output_data(Default::default())
                .cell_dep(node_fork2021.always_success_cell_dep())
                .build()
        };
        let since_relative_epoch_number_with_fraction1 =
            since_from_relative_epoch_number_with_fraction(EpochNumberWithFraction::new_unchecked(
                0, 1801, 1800,
            ));
        let since_relative_epoch_number_with_fraction2 =
            since_from_relative_epoch_number_with_fraction(EpochNumberWithFraction::new_unchecked(
                0, 1800, 1800,
            ));
        let since_absolute_epoch_number_with_fraction1 =
            since_from_absolute_epoch_number_with_fraction(EpochNumberWithFraction::new_unchecked(
                current_block_epoch.number(),
                2,
                1,
            ));
        let since_absolute_epoch_number_with_fraction2 =
            since_from_absolute_epoch_number_with_fraction(EpochNumberWithFraction::new_unchecked(
                current_block_epoch.number(),
                1,
                1,
            ));
        let txs = vec![
            build_transaction(since_relative_epoch_number_with_fraction1, &cells[0]),
            build_transaction(since_relative_epoch_number_with_fraction2, &cells[1]),
            build_transaction(since_absolute_epoch_number_with_fraction1, &cells[2]),
            build_transaction(since_absolute_epoch_number_with_fraction2, &cells[3]),
        ];

        // Move forward to make sure our since values become valid
        node_fork2021.mine(1800 + 10);

        assert!(!is_rfc0223_switched(node_fork2021));
        txs.iter().enumerate().for_each(|(i, tx)| {
            let result = node_fork2021
                .rpc_client()
                .send_transaction_result(tx.pack().data().into());
            assert!(
                result.is_ok(),
                "node_fork2021 should accept tx-{} according to old rule, but got: {:?}",
                i,
                result
            );
        });
        node_fork2021.mine(3);
        assert!(!is_rfc0223_switched(node_fork2021));
        nodes
            .waiting_for_sync()
            .expect("nodes should be synced as they all abey to old rule");

        assert!(txs
            .iter()
            .all(|tx| node_fork2021.is_transaction_committed(tx)));
    }
}

fn is_rfc0223_switched(node: &Node) -> bool {
    node.rpc_client().get_current_epoch().number.value() >= RFC0223_EPOCH_NUMBER
}