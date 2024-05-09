wrk.method = "POST"
wrk.headers["Content-Type"] = "application/json"

wrk.body = [[
{
    "id": 2,
    "jsonrpc": "2.0",
    "method": "get_cells",
    "params": [
        {
            "script": {
                "code_hash": "0x709f3fda12f561cfacf92273c57a98fede188a3f1a59b1f888d113f9cce08649",
                "hash_type": "data",
                "args": "0x1e2358fce76b94a15b50284e06d93ed028eb5c80"
            },
            "script_type": "lock",
            "script_search_mode": "exact"
        },
        "asc",
        "0x64"
    ]
}
]]


function response(status, headers, body)
    if (string.find(body, '"error"')) then
        print('error, resp: ', body)
        wrk.thread:stop()
    end
end