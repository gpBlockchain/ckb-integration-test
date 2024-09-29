### ckb bench conflict tx


#### Test Scenario

0. Pull the latest development version of CKB binary.
1. Deploy 3 nodes.
2. all nodes starts the miner program, mining one block every 8 second.
3. Connect the 3 nodes.
4. Use ckb-bench to prepare 20,000 cells.
5. Perform a 30-minute stress test using these 20,000 cells, with the following test scenarios:
    1. Transfer one cell use diffrent fee at a transaction with a concurrency of 4.

