### ckb bench with data

#### Test Scenario

0. Pull the latest development version of CKB binary.
1. Deploy 3 nodes with block data
    1. 100 million data
    2. 200 million data
2. Connect the 3 nodes.
3. Perform a 30-minute stress test using these 10,000 cells, with the following test scenarios:
    1. Use `ckb-bench` to alternately let each node mine blocks  every `8` second  and then stress test `node 2` with different TPS rates:
       1. 10 tps
       2. 150 tps
    2. On node 2, use `ckb-miner` to mine a block every `1` second, then stress test node 2 with 8 threads.

#### Interpretation of Test Results

The test results will be synchronized to the following [link](https://github.com/nervosnetwork/ckb-integration-test/issues/116).
Explain the names of all except ckb-bench-server.

<table>
    <tr>
        <th colspan="1">Key</th><th>Value</th>
    </tr>

   <tr>
        <td colspan="1">block_tip_number</td><td>Height of CKB Restart</td>
    </tr>
    <tr>
        <td colspan="1">wait_restart_rpc_cost_time</td><td> Using Ansible to log Wait ckb rpc server start cost time</td>
    </tr>
</table>
