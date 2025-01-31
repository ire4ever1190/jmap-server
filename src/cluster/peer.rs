/*
 * Copyright (c) 2020-2022, Stalwart Labs Ltd.
 *
 * This file is part of the Stalwart JMAP Server.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of
 * the License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 * in the LICENSE file at the top-level directory of this distribution.
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 *
 * You can be released from the requirements of the AGPLv3 license by
 * purchasing a commercial license. Please contact licensing@stalw.art
 * for more details.
*/

use super::{
    gossip::PeerInfo, rpc::peer::spawn_peer_rpc, Cluster, Peer, PeerId, ShardId, HEARTBEAT_WINDOW,
};
use std::{fmt::Display, net::SocketAddr, time::Instant};
use store::Store;

impl Peer {
    pub fn new_seed<T>(cluster: &Cluster<T>, peer_id: PeerId, addr: SocketAddr) -> Self
    where
        T: for<'x> Store<'x> + 'static,
    {
        let (tx, online_rx) = spawn_peer_rpc(
            cluster.tx.clone(),
            cluster.peer_id,
            &cluster.config,
            peer_id,
            addr,
        );
        Peer {
            peer_id,
            shard_id: 0,
            tx,
            online_rx,
            epoch: 0,
            generation: 0,
            addr,
            state: crate::cluster::gossip::State::Seed,
            hostname: "".to_string(),
            last_heartbeat: Instant::now(),
            hb_window: vec![0; HEARTBEAT_WINDOW],
            hb_window_pos: 0,
            hb_sum: 0,
            hb_sq_sum: 0,
            hb_is_full: false,
            last_log_index: 0,
            last_log_term: 0,
            commit_index: 0,
            vote_granted: false,
        }
    }

    pub fn new<T>(
        cluster: &Cluster<T>,
        peer: PeerInfo,
        state: crate::cluster::gossip::State,
    ) -> Self
    where
        T: for<'x> Store<'x> + 'static,
    {
        let (tx, online_rx) = spawn_peer_rpc(
            cluster.tx.clone(),
            cluster.peer_id,
            &cluster.config,
            peer.peer_id,
            peer.addr,
        );
        Peer {
            peer_id: peer.peer_id,
            shard_id: peer.shard_id,
            tx,
            online_rx,
            epoch: peer.epoch,
            generation: peer.generation,
            addr: peer.addr,
            hostname: peer.hostname,
            state,
            last_heartbeat: Instant::now(),
            hb_window: vec![0; HEARTBEAT_WINDOW],
            hb_window_pos: 0,
            hb_sum: 0,
            hb_sq_sum: 0,
            hb_is_full: false,
            last_log_index: peer.last_log_index,
            last_log_term: peer.last_log_term,
            commit_index: peer.last_log_index,
            vote_granted: false,
        }
    }

    pub fn is_seed(&self) -> bool {
        self.state == crate::cluster::gossip::State::Seed
    }

    pub fn is_alive(&self) -> bool {
        self.state == crate::cluster::gossip::State::Alive
    }

    pub fn is_suspected(&self) -> bool {
        self.state == crate::cluster::gossip::State::Suspected
    }

    pub fn is_healthy(&self) -> bool {
        matches!(
            self.state,
            crate::cluster::gossip::State::Alive | crate::cluster::gossip::State::Suspected
        )
    }

    pub fn is_offline(&self) -> bool {
        matches!(
            self.state,
            crate::cluster::gossip::State::Offline | crate::cluster::gossip::State::Left
        )
    }

    pub fn is_in_shard(&self, shard_id: ShardId) -> bool {
        self.shard_id == shard_id
    }
}

impl Display for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.addr)
    }
}

impl<T> Cluster<T>
where
    T: for<'x> Store<'x> + 'static,
{
    pub fn is_peer_healthy(&self, peer_id: PeerId) -> bool {
        self.peers
            .iter()
            .any(|p| p.peer_id == peer_id && p.is_healthy())
    }

    pub fn get_peer(&self, peer_id: PeerId) -> Option<&Peer> {
        self.peers.iter().find(|p| p.peer_id == peer_id)
    }

    pub fn is_known_peer(&self, peer_id: PeerId) -> bool {
        self.peers.iter().any(|p| p.peer_id == peer_id)
    }

    pub fn get_peer_mut(&mut self, peer_id: PeerId) -> Option<&mut Peer> {
        self.peers.iter_mut().find(|p| p.peer_id == peer_id)
    }
}
