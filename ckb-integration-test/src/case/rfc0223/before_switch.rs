use crate::case::{Case, CaseOptions};
use crate::{CKB2019, CKB2021};
use ckb_testkit::node::{Node, NodeOptions};
use ckb_testkit::nodes::Nodes;
use ckb_testkit::util::{
    since_from_absolute_epoch_number_with_fraction, since_from_relative_epoch_number_with_fraction,
};
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
                    node_name: "node2019",
                    ckb_binary: CKB2019.read().unwrap().clone(),
                    initial_database: "testdata/db/empty",
                    chain_spec: "testdata/spec/ckb2019",
                    app_config: "testdata/config/ckb2019",
                },
                NodeOptions {
                    node_name: "node2021",
                    ckb_binary: CKB2021.read().unwrap().clone(),
                    initial_database: "testdata/db/empty",
                    chain_spec: "testdata/spec/ckb2021",
                    app_config: "testdata/config/ckb2021",
                },
            ]
            .into_iter()
            .collect(),
        }
    }

    fn run(&self, nodes: Nodes) {
        let node2019 = nodes.get_node("node2019");
        let node2021 = nodes.get_node("node2021");
        node2019.mine(1);
        node2021.mine(1);
        nodes.p2p_connect();
        node2021.mine(node2021.consensus().tx_proposal_window.farthest.value() + 4);
        nodes.waiting_for_sync().expect("nodes should be synced");

        let current_block_epoch = node2021.get_tip_block().epoch();
        let cells = node2019.get_live_always_success_cells();
        assert!(cells.len() >= 4);

        let build_transaction = |input: &CellMeta, since: u64| {
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
                .cell_dep(node2021.always_success_cell_dep())
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
            build_transaction(&cells[0], since_relative_epoch_number_with_fraction1),
            build_transaction(&cells[1], since_relative_epoch_number_with_fraction2),
            build_transaction(&cells[2], since_absolute_epoch_number_with_fraction1),
            build_transaction(&cells[3], since_absolute_epoch_number_with_fraction2),
        ];

        // Move forward to make sure our since values become valid
        node2021.mine(1800 + 10);

        assert!(!is_rfc0223_switched(node2021));
        txs.iter().enumerate().for_each(|(i, tx)| {
            let result = node2021
                .rpc_client()
                .send_transaction_result(tx.pack().data().into());
            assert!(
                result.is_ok(),
                "node2021 should accept tx-{} according to old rule, but got: {:?}",
                i,
                result
            );
        });
        node2021.mine(3);
        assert!(!is_rfc0223_switched(node2021));
        nodes
            .waiting_for_sync()
            .expect("nodes should be synced as they all abey to old rule");

        assert!(txs.iter().all(|tx| node2021.is_transaction_committed(tx)));
    }
}

fn is_rfc0223_switched(node: &Node) -> bool {
    node.rpc_client().get_current_epoch().number.value() >= RFC0223_EPOCH_NUMBER
}