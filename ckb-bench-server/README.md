### ckb bench server

#### Test Scenario

0. Pull the latest development version of CKB binary.
1. Deploy 3 nodes.
2. Node 2 starts the miner program, mining one block every 1 second.
3. Connect the 3 nodes.
4. Use ckb-bench to prepare 20,000 cells.
5. Perform a 30-minute stress test using these 20,000 cells, with the following test scenarios:
    1. Transfer one cell at a transaction with a concurrency of 8.
    2. Transfer two cells at a transaction with a concurrency of 8.
    3. Transfer five cells at a transaction with a concurrency of 8.
    4. Transfer ten cells at a transaction with a concurrency of 8.

#### Interpretation of Test Results

The test results will be synchronized to the following [link](https://github.com/nervosnetwork/ckb-integration-test/issues/116).
<table>
    <tr>
        <th colspan="3">Key</th><th>Value</th>
    </tr>

   <tr>
        <td colspan="3">n_nodes</td><td>Number of running CKB nodes</td>
    </tr>
    <tr>
        <td colspan="3">n_inout</td><td>Number of transaction inputs and outputs</td>
    </tr>
    <tr>
        <td colspan="3">ckb_version</td><td>Client version of the running CKB nodes</td>
    </tr>
    <tr>
        <td colspan="3">delay_time_ms</td><td>Delay time between sending continuous transactions, equal to `--tx-interval-ms` (Note: This parameter does not take effect when specifying TPS stress testing, i.e., when `set_client_tps` is not equal to 0)</td>
    </tr>
    <tr>
        <td colspan="3">from_block_number</td><td>The chain height when starting benchmark (Statistics)</td>
    </tr>
    <tr>
        <td colspan="3">to_block_number</td><td>The chain height when ending benchmark (Statistics)</td>
    </tr>
    <tr>
        <td colspan="3">transactions_per_second</td><td>On-chain transactions per second</td>
    </tr>
    <tr>
        <td colspan="3">average_block_transactions</td><td>Average block transactions</td>
    </tr>
    <tr>
        <td colspan="3">average_block_time_ms</td><td>Average block interval in milliseconds</td>
    </tr>
    <tr>
        <td colspan="3">total_transactions</td><td>Total transactions</td>
    </tr>
    <tr>
        <td colspan="3">total_transactions_size</td><td>Total transaction size</td>
    </tr>
    <tr>
        <td colspan="3">set_send_tps</td><td>Client send TPS</td>
    </tr>
    <tr>
        <td colspan="3">set_send_tps</td><td>Client set send TPS, equivalent to `--tps`</td>
    </tr>
    <tr>
        <td colspan="3">client_send_tps</td><td>Client send actual TPS</td>
    </tr>
    <tr>
        <td colspan="3">grafana</td><td>Prometheus data link when CKB nodes are running</td>
    </tr>
    <tr>
        <td rowspan="13">report</td><td colspan="2">Total Report</td><td>Test data above</td>
    </tr>
    <tr>
        <td rowspan="2">Run Report</td><td>tps</td><td>Client-sent TPS</td>
    </tr>
    <tr>
        <td>delay_ms</td><td>Client transaction sending delay</td>
    </tr>
    <tr>
        <td rowspan="4">Pool Report</td><td>pending</td><td>Pending pool</td>
    </tr>
    <tr>
        <td>orphan</td><td>Orphan pool</td>
    </tr>
    <tr>
        <td>proposed</td><td>Proposed pool</td>
    </tr>
    <tr>
        <td>block_number</td><td>Block number</td>
    </tr>
    <tr>
        <td rowspan="4">Block Report</td><td>block_delay_ms</td><td>On-chain block creation delay</td>
    </tr>
    <tr>
        <td>tps</td><td>On-chain TPS</td>
    </tr>
    <tr>
        <td>block_transaction_size</td><td>Block transaction count</td>
    </tr>
    <tr>
        <td>block_number</td><td>Block height</td>
    </tr>
    <tr>
        <td rowspan="2">Block Report</td><td>ckb_sys_mem_process_rss_mb</td><td>CKB Prometheus RSS data, when `--prometheus` is set</td>
    </tr>
    <tr>
        <td>ckb_sys_mem_process_vms_mb</td><td>CKB Prometheus VMS data, when `--prometheus` is set</td>
    </tr>
</table>
