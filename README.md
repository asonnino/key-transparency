# Key Transparency

[![build status](https://img.shields.io/github/workflow/status/asonnino/key-transparency/Rust/master?style=flat-square&logo=github)](https://github.com/asonnino/key-transparency/actions)
[![rustc](https://img.shields.io/badge/rustc-1.64+-blue?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![license](https://img.shields.io/badge/license-Apache-blue.svg?style=flat-square)](LICENSE)

This repo provides an prototype implementation of [SYSNAME](), based on [akd](https://github.com/novifinancial/akd). The codebase has been designed to be small, efficient, and easy to benchmark and modify. It has not been designed to run in production but uses real cryptography ([dalek](https://doc.dalek.rs/ed25519_dalek)), networking ([tokio](https://docs.rs/tokio)), and storage ([rocksdb](https://docs.rs/rocksdb)).

## Quick Start

The core protocols are written in Rust, but all benchmarking scripts are written in Python and run with [Fabric](http://www.fabfile.org/).
To deploy and benchmark a testbed of 4 witnesse on your local machine, clone the repo and install the python dependencies:

```
git clone https://github.com/asonnino/key-transparency.git
cd key-transparency/scripts
```

It is advised to install the python dependencies in a virtual environment such as [virtualenv](https://pypi.org/project/virtualenv):

```
virtualenv venv
source venv/bin/activate
pip install -r requirements.txt
```

You also need to install Clang (required by rocksdb) and [tmux](https://linuxize.com/post/getting-started-with-tmux/#installing-tmux) (which runs all nodes and clients in the background). Finally, run a local benchmark using fabric:

```
fab local
```

This command may take a long time the first time you run it (compiling rust code in `release` mode may be slow) and you can customize a number of benchmark parameters in `fabfile.py`. When the benchmark terminates, it displays a summary of the execution similarly to the one below.

```
-----------------------------------------
 SUMMARY:
-----------------------------------------
 + CONFIG:
 Faults: 0 node(s)
 Committee size: 4 node(s)
 Shard(s) per node: 1 shard(s)
 Collocate shards: True
 Batch size: 100
 Input rate: 1,000 tx/s
 Execution time: 20 s

 + RESULTS:
 Client TPS: 0 tx/s
 Client latency: 0 ms
 IdP TPS: 1,024 tx/s
 IdP latency: 279 ms
 End-to-end TPS: 1,024 tx/s
 End-to-end latency: 280 ms
-----------------------------------------
```

## Micro-benchmarks

The following command micro-benchmarks the main functions of the IdP and witnesses on your local machine:

```
cargo run --features=micro-benchmark --release --bin micro_benchmark
```

## License

This software is licensed as [Apache 2.0](LICENSE).
