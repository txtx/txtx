
# Setup steps
1. Generate rollup.json/genesis.json/jwt.txt
   1. Write to host disk
2. Take genesis.json and use it to initialize geth: this writes to a datadir
   1. Read from host, write to host datadir

# Running nodes
3. Start geth, using datadir + jwt.txt
   1. Copy host datadir + jwt, only write to internal docker disk?
      1. We need to create a volume that's not mounted to host, but takes host disk data as starting source
4. Start node, using rollup.json + jwt.txt
   1. Copy host rollup + jwt, only write to internal docker disk?
      1. We need to create a volume that's not mounted to host, but takes host disk data as starting source

# Contract Deployments
These write to geth's datadir






jwt.txt - user already created?
6 containers:
l2_config_file_gen_container* -> creates `rollup.json`/`genesis.json` (op-node code)
geth_init_container* - uses `genesis.json`, creates `datadir` (op-geth code)
op_geth - read/write `datadir` + read `jwt.txt` 
    pull from host `datadir` when starting, but subsequent writes should not be mounted to host, they should be internal to the container only
op_node - read `rollup.json` + read `jwt.txt`
    pull from `rollup.json`, but it should not be updated ever
op_batcher
op_proposer

To package images:
commit op_geth -> snapshot of datadir + command used to start
commit op_node -> snapshot of command used to start, with args
commit op_batcher -> snapshot of command used to start, with args
commit op_proposer -> snapshot of command used to start, with args



* run to completion, then can be killed



To investigate:
    is there a way with geth to "unlock" accounts to allow for future signing?

rollup.json:
```json
{
  "genesis": {
    "l1": {
      "hash": "0xf7c8eceef8412ccd9f06fb799133ba66dbfeb17529b69f83a11331169a749935",
      "number": 7011690
    },
    "l2": {
      "hash": "0x97873414adfa9cc223e6d294d585677f061cb27e75b1cbfeef1875bb2c3ae754",
      "number": 0
    },
    "l2_time": 1730743668,
    "system_config": {
      "batcherAddr": "0xbbfe9114e6159c89571f0c8d7d2c177fcee51b4e",
      "overhead": "0x0000000000000000000000000000000000000000000000000000000000000834",
      "scalar": "0x00000000000000000000000000000000000000000000000000000000000f4240",
      "gasLimit": 30000000,
      "baseFeeScalar": 0,
      "blobBaseFeeScalar": 0
    }
  },
  "block_time": 2,
  "max_sequencer_drift": 600,
  "seq_window_size": 3600,
  "channel_timeout": 300,
  "l1_chain_id": 11155111,
  "l2_chain_id": 42069,
  "regolith_time": 0,
  "canyon_time": 0,
  "batch_inbox_address": "0xff00000000000000000000000000000000042069",
  "deposit_contract_address": "0xe8dda55110c9a765bc665782992c41b49b9f8903",
  "l1_system_config_address": "0xd175d46c2016e8f4ac49db91d929134fb38dedae",
  "protocol_versions_address": "0x0000000000000000000000000000000000000000"
}

```



---

Running rollup output:
## Copy the volumes to your destination
```sh
cp /working/dir/datadir.tar.gq /dest/dir
cp /working/dir/conf.tar.gq /dest/dir
cd /dest/dir
docker volume create datadir
docker volume create conf
# unpack the contents of datadir.tar.gz and add to datadir volume
docker run --rm -v datadir:/datadir -v ./datadir.tar.gz:/backup/datadir.tar.gz busybox tar xzf /backup/datadir.tar.gz -C /datadir --strip-components=1
# unpack the contents of conf.tar.gz and add to conf volume
docker run --rm -v conf:/conf -v ./conf.tar.gz:/backup/conf.tar.gz busybox tar xzf /backup/conf.tar.gz -C /conf --strip-components=1
# create a network
docker network create my-network
# start op-geth
docker run --network my-network -v datadir:/datadir -v conf:/conf op-geth:txtx-latest
# start op-node
docker run --network my-network -v conf:/conf op-node:txtx-latest
# start op-batcher
docker run --network my-network op-batcher:txtx-latest
# start op-proposer
docker run --network my-network op-proposer:txtx-latest
```