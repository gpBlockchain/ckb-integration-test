wrk.method = "POST"
wrk.headers["Content-Type"] = "application/json"

wrk.body = [[
{
    "id": 2,
    "jsonrpc": "2.0",
    "method": "get_cells_capacity",
    "params": [
        {
            "script": {
                "code_hash": "0x709f3fda12f561cfacf92273c57a98fede188a3f1a59b1f888d113f9cce08649",
                "hash_type": "data",
                "args": "0x30e87dd4b3d46bbd1521c3efb3405e0693669831"
            },
            "script_type": "lock",
            "script_search_mode": "exact"
        }
    ]
}
]]


function response(status, headers, body)
    if (string.find(body, '"error"')) then
        print('error, resp: ', body)
        wrk.thread:stop()
    end
end

-- This command is run under the condition that the CPU has 4 cores
-- wrk -t4 -c100 -d60s -s ./util/rich-indexer/src/tests/stress_test_scripts/get_cells_capacity_exact.lua --latency http://127.0.0.1:8114
