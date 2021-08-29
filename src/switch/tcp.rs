//! # TCP Adapter
//!
//! Default adapter that uses rusts builtin TcpListener to receive new
//! messages. This is currently the only included adapter, in the
//! future this selection will hopefully get expanded.

use super::adapter;

pub struct TcpAdapter {
    mode: adapter::Mode,
}
