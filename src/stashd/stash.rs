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

use std::collections::{BTreeSet, VecDeque};

use bitcoin::hashes::Hash;
use lnpbp::client_side_validation::Conceal;
use lnpbp::seals::{OutpointHash, OutpointReveal};
use rgb::{
    Anchor, Assignments, AutoConceal, Consignment, ContractId, Disclosure,
    Extension, Genesis, Node, NodeId, SchemaId, SealEndpoint, Stash, Transition,
};

use super::index::Index;
use super::storage::Store;
use super::Runtime;

#[derive(Clone, PartialEq, Eq, Debug, Display, From, Error)]
#[display(Debug)]
pub enum Error {
    #[from(super::storage::DiskStorageError)]
    StorageError,

    #[from(super::index::BTreeIndexError)]
    IndexError,

    AnchorParameterIsRequired,

    GenesisNode,
}

pub struct DumbIter<T>(std::marker::PhantomData<T>);
impl<T> Iterator for DumbIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}

impl Stash for Runtime {
    type Error = Error;
    type GenesisIterator = DumbIter<Genesis>;
    type AnchorIterator = DumbIter<Anchor>;
    type TransitionIterator = DumbIter<Transition>;
    type ExtensionIterator = DumbIter<Extension>;
    type NidIterator = DumbIter<NodeId>;

    fn get_schema(
        &self,
        _schema_id: SchemaId,
    ) -> Result<SchemaId, Self::Error> {
        unimplemented!()
    }

    fn get_genesis(
        &self,
        _contract_id: ContractId,
    ) -> Result<Genesis, Self::Error> {
        unimplemented!()
    }

    fn get_transition(
        &self,
        _node_id: NodeId,
    ) -> Result<Transition, Self::Error> {
        unimplemented!()
    }

    fn get_extension(
        &self,
        _node_id: NodeId,
    ) -> Result<Extension, Self::Error> {
        unimplemented!()
    }

    fn get_anchor(
        &self,
        _anchor_id: ContractId,
    ) -> Result<Anchor, Self::Error> {
        unimplemented!()
    }

    fn genesis_iter(&self) -> Self::GenesisIterator {
        unimplemented!()
    }

    fn anchor_iter(&self) -> Self::AnchorIterator {
        unimplemented!()
    }

    fn transition_iter(&self) -> Self::TransitionIterator {
        unimplemented!()
    }

    fn extension_iter(&self) -> Self::ExtensionIterator {
        unimplemented!()
    }

    fn consign(
        &self,
        contract_id: ContractId,
        node: &impl Node,
        anchor: Option<&Anchor>,
        expose: &BTreeSet<SealEndpoint>,
    ) -> Result<Consignment, Error> {
        let genesis = self.storage.genesis(&contract_id)?;
        let concealed_endpoints =
            expose.iter().map(SealEndpoint::conceal).collect();

        let mut state_transitions = vec![];
        let mut state_extensions: Vec<Extension> = vec![];
        if let Some(transition) =
            node.as_any().downcast_ref::<Transition>().clone()
        {
            let mut transition = transition.clone();
            transition.conceal_except(&concealed_endpoints);
            let anchor = anchor.ok_or(Error::AnchorParameterIsRequired)?;
            state_transitions.push((anchor.clone(), transition.clone()));
        } else if let Some(extension) =
            node.as_any().downcast_ref::<Extension>().clone()
        {
            let mut extension = extension.clone();
            extension.conceal_except(&concealed_endpoints);
            state_extensions.push(extension.clone());
        } else {
            Err(Error::GenesisNode)?;
        }

        let mut sources = VecDeque::<NodeId>::new();
        sources
            .extend(node.parent_owned_rights().into_iter().map(|(id, _)| id));
        sources
            .extend(node.parent_public_rights().into_iter().map(|(id, _)| id));
        while let Some(node_id) = sources.pop_front() {
            if node_id.into_inner() == genesis.contract_id().into_inner() {
                continue;
            }
            let anchor_id = self.indexer.anchor_id_by_transition_id(node_id)?;
            let anchor = self.storage.anchor(&anchor_id)?;
            // TODO: (new) Improve this logic
            match (
                self.storage.transition(&node_id),
                self.storage.extension(&node_id),
            ) {
                (Ok(mut transition), Err(_)) => {
                    transition.conceal_all();
                    state_transitions.push((anchor, transition.clone()));
                    sources.extend(
                        transition
                            .parent_owned_rights()
                            .into_iter()
                            .map(|(id, _)| id),
                    );
                    sources.extend(
                        transition
                            .parent_public_rights()
                            .into_iter()
                            .map(|(id, _)| id),
                    );
                }
                (Err(_), Ok(mut extension)) => {
                    extension.conceal_all();
                    state_extensions.push(extension.clone());
                    sources.extend(
                        extension
                            .parent_owned_rights()
                            .into_iter()
                            .map(|(id, _)| id),
                    );
                    sources.extend(
                        extension
                            .parent_public_rights()
                            .into_iter()
                            .map(|(id, _)| id),
                    );
                }
                _ => Err(Error::StorageError)?,
            }
        }

        let node_id = node.node_id();
        let endpoints = expose.iter().map(|op| (node_id, *op)).collect();
        Ok(Consignment::with(
            genesis,
            endpoints,
            state_transitions,
            state_extensions,
        ))
    }

    fn merge(
        &mut self,
        consignment: &Consignment,
        known_seals: &Vec<OutpointReveal>,
    ) -> Result<(), Error> {
        // [PRIVACY]:
        // Update transition data with the revealed state information that we
        // kept since we did an invoice (and the sender did not know).
        let reveal_known_seals =
            |(_, assignments): (&usize, &mut Assignments)| match assignments {
                Assignments::Declarative(_) => {}
                Assignments::DiscreteFiniteField(set) => {
                    *set = set
                        .iter()
                        .map(|a| {
                            let mut a = a.clone();
                            a.reveal_seals(known_seals.iter());
                            a
                        })
                        .collect();
                }
                Assignments::CustomData(set) => {
                    *set = set
                        .iter()
                        .map(|a| {
                            let mut a = a.clone();
                            a.reveal_seals(known_seals.iter());
                            a
                        })
                        .collect();
                }
            };

        for (anchor, transition) in consignment.state_transitions.iter() {
            let mut transition = transition.clone();
            transition
                .owned_rights_mut()
                .into_iter()
                .for_each(reveal_known_seals);
            // Store the transition and the anchor data in the stash
            self.storage.add_anchor(&anchor)?;
            self.storage.add_transition(&transition)?;
        }

        for extension in consignment.state_extensions.iter() {
            let mut extension = extension.clone();
            extension
                .owned_rights_mut()
                .into_iter()
                .for_each(reveal_known_seals);
            self.storage.add_extension(&extension)?;
        }

        Ok(())
    }

    fn forget(
        &mut self,
        _consignment: Consignment,
    ) -> Result<usize, Self::Error> {
        unimplemented!()
    }

    fn prune(&mut self) -> Result<usize, Self::Error> {
        unimplemented!()
    }

    fn disclose(&self) -> Result<Disclosure, Self::Error> {
        unimplemented!()
    }
}
