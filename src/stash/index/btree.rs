// RGB standard library
// Written in 2020 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License
// along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use std::collections::BTreeMap;

use super::Index;

#[derive(Display, Debug)]
#[display_from(Debug)]
pub struct BtreeIndex {
    index: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl BtreeIndex {
    pub fn new() -> Self {
        Self { index: bmap! {} }
    }
}

impl Index for BtreeIndex {}
