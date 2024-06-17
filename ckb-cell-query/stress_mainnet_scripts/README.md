## 针对 Rich-Indexer 和indexer 做的性能测试场景

直接使用 主网数据来做针对性的压测

接下来会从如下6种场景设计大数据量压测场景

### `get_cells` script args partial mode search

根据rgb持有人数最多的token设计如下场景

- 查询otter
  token信息: [otter](https://explorer.nervos.org/xudt/0x7aa08eb97ac10170b60666b7494af5692757bb23e1a89a2afea3d0172cdcff50)
    - [get_cells.exact.0x64.otter.lua](get_cells.exact.0x64.otter.lua)
    - [get_cells.partial.0x64.otter.lua](get_cells.partial.0x64.otter.lua)

根据ckb cell持有人数设计场景

- 查询 SECP256K1
  cell [SECP256K1](https://explorer.nervos.org/script/0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8/type)
    - [get_cells.partial.0x64.secp256k1.0x.lua](get_cells.partial.0x64.secp256k1.0x.lua)(sqlite time out)
- 查询 omni
  cell: [omni_lock](https://explorer.nervos.org/script/0x9b819793a64463aed77c615d6cb226eea5487ccfc0783043a587254cda2b6f26/type)
    - [get_cells.partial.0x64.omni.0x.lua](get_cells.partial.0x64.omni.0x.lua)
- 查询joyid
  总cell: [joyid](https://explorer.nervos.org/script/0xd00c84f0ec8fd441c38bc3f87a371f547190f2fcff88e642bc5bf54b9e318323/type)
    - [get_cells.partial.0x64.joyid.0x.lua](get_cells.partial.0x64.joyid.0x.lua)(sqlite time out:59s)
- 查询xudt
  总cell: [xudt](https://explorer.nervos.org/script/0x50bd8d6680b8b9cf98b73f3c08faf8b2a21914311954118ad6609be6e78a1b95/data1)
    - [get_cells.partial.0x64.xudt.0x.lua](get_cells.partial.0x64.xudt.0x.lua)
- 查询矿工cell 信息
    - [get_cells.partial.0x64.secp256k1.miner.lua](get_cells.partial.0x64.secp256k1.miner.lua)

### `get_cells` cell data filter(prefix|exact|partial)

- 查询xudt信息
    - [get_cells.partial.0x64.data.filter.partial.xudt.0x.lua](get_cells.partial.0x64.data.filter.partial.xudt.0x.lua)
    - [get_cells.prefix.0x64.data.filter.partial.xudt.0x.lua](get_cells.prefix.0x64.data.filter.partial.xudt.0x.lua)

### `get_transactions` script args partial mode search

- 查询 omiga 压测期间信息
    - [get_transactions.partial.0x1.omiga.block_range.0xc36e31.0xc36e3d.lua](get_transactions.partial.0x1.omiga.block_range.0xc36e31.0xc36e3d.lua)
    - [get_transactions.partial.0x1.omiga.block_range.0x01.0xffffffffff.lua](get_transactions.partial.0x1.omiga.block_range.0x01.0xffffffffff.lua)
- 查询 SECP256K1
    - [get_transactions.partial.0x64.secp256k1.0x.lua](get_transactions.partial.0x64.secp256k1.0x.lua)(sqlite:timeout )

### `get_transactions` cell data filter(prefix|exact|partial)

- 查询 omiga 压测期间信息
    - [get_transactions.prefix.0x1.omiga.block_range.0x01.0xffffffffff.lua](get_transactions.prefix.0x1.omiga.block_range.0x01.0xffffffffff.lua)
    - [get_transactions.prefix.0x1.omiga.block_range.0xc36e31.0xc36e3d.lua](get_transactions.prefix.0x1.omiga.block_range.0xc36e31.0xc36e3d.lua)

### `get_cells_capacity` script args partial mode search

- 查询 SECP256K1 总cell
    - [get_cells_capacity.partial.secp256k1.0x.lua](get_cells_capacity.partial.secp256k1.0x.lua)(sqlite:timeout)
- 查询 omni 总cell
    - [get_cells_capacity.partial.omni.0x.lua](get_cells_capacity.partial.omni.0x.lua)
- 查询joyid 总cell
    - [get_cells_capacity.partial.joyid.0x.lua](get_cells_capacity.partial.joyid.0x.lua)
- 查询xudt 总cell
    - [get_cells_capacity.partial.xudt.0x.lua](get_cells_capacity.partial.xudt.0x.lua)
- 查询矿工cell 信息
    - [get_cells_capacity.partial.secp256k1.miner.lua](get_cells_capacity.partial.secp256k1.miner.lua)
    - [get_cells_capacity.exact.secp256k1.miner.lua](get_cells_capacity.exact.secp256k1.miner.lua)

### get_cells_capacity cell data filter(prefix|exact|partial)
- 查询xudt 总cell
  - [get_cells_capacity.partial.data.filter.partial.xudt.0x.lua](get_cells_capacity.partial.data.filter.partial.xudt.0x.lua)
  - [get_cells_capacity.prefix.data.filter.partial.xudt.0x.lua](get_cells_capacity.prefix.data.filter.partial.xudt.0x.lua)