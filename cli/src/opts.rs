// RGB node providing smart contracts functionality for Bitcoin & Lightning.
//
// Written in 2022 by
//     Dr. Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// Copyright (C) 2022 by LNP/BP Standards Association, Switzerland.
//
// You should have received a copy of the MIT License along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use std::path::PathBuf;

use clap::ValueHint;
use internet2::addr::ServiceAddr;
use rgb::Contract;
use rgb_rpc::{RGB_NODE_DATA_DIR, RGB_NODE_RPC_ENDPOINT};

/// Command-line tool for working with RGB node
#[derive(Parser, Clone, PartialEq, Eq, Debug)]
#[clap(name = "rgb-cli", bin_name = "rgb-cli", author, version)]
pub struct Opts {
    /// ZMQ socket for connecting daemon RPC interface.
    ///
    /// Socket can be either TCP address in form of `<ipv4 | ipv6>:<port>` – or a path
    /// to an IPC file.
    ///
    /// Defaults to `127.0.0.1:62962`.
    #[clap(
        short,
        long,
        global = true,
        default_value = RGB_NODE_RPC_ENDPOINT,
        env = "RGB_NODE_RPC_ENDPOINT"
    )]
    pub connect: ServiceAddr,

    /// Data directory path.
    ///
    /// Path to the directory that contains stored data, and where ZMQ RPC
    /// socket files are located
    #[clap(
        short,
        long,
        global = true,
        default_value = RGB_NODE_DATA_DIR,
        env = "RGB_NODE_DATA_DIR",
        value_hint = ValueHint::DirPath
    )]
    pub data_dir: PathBuf,

    /// Set verbosity level.
    ///
    /// Can be used multiple times to increase verbosity.
    #[clap(short, long, global = true, parse(from_occurrences))]
    pub verbose: u8,

    /// Command to execute
    #[clap(subcommand)]
    pub command: Command,
}

/// Command-line commands:
#[derive(Subcommand, Clone, PartialEq, Eq, Debug, Display)]
pub enum Command {
    /// Add new contract to the node
    #[display("register(...)")]
    Register { contract: Contract },
}
