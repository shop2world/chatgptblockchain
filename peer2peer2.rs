use super::{앱, 블록};
use libp2p::{
    floodsub::{Floodsub, FloodsubEvent, Topic},
    identity,
    mdns::{Mdns, MdnsEvent},
    swarm::{NetworkBehaviourEventProcess, Swarm},
    NetworkBehaviour, PeerId,
};
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::sync::mpsc;




pub static KEYS: Lazy<identity::Keypair> = Lazy::new(identity::Keypair::generate_ed25519);
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("블록들"));

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainResponse {
    pub 블록들: Vec<블록>,
    pub receiver: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct 로칼_체인_요청_구조체 {
    pub 출처_peer_id: String,
}

pub enum 이벤트_유형_열거형_데이타 {
    로컬_체인_반응(ChainResponse),
    Input(String),
    Init,
}

#[derive(NetworkBehaviour)]
pub struct AppBehaviour {
    pub floodsub: Floodsub,
    pub mdns: Mdns,
    #[behaviour(ignore)]
    pub 반응_송신자: mpsc::UnboundedSender<ChainResponse>,
    #[behaviour(ignore)]
    pub 초기_송신자: mpsc::UnboundedSender<bool>,
    #[behaviour(ignore)]
    pub app: 앱,
}

impl AppBehaviour {
    pub async fn new(
        app: 앱,
        반응_송신자: mpsc::UnboundedSender<ChainResponse>,
        초기_송신자: mpsc::UnboundedSender<bool>,
    ) -> Self {
        let mut 처리_하자 = Self {
            app,
            floodsub: Floodsub::new(*PEER_ID),
            mdns: Mdns::new(Default::default())
                .await
                .expect("can create mdns"),
            반응_송신자,
            초기_송신자,
        };
        처리_하자.floodsub.subscribe(CHAIN_TOPIC.clone());
        처리_하자.floodsub.subscribe(BLOCK_TOPIC.clone());

        처리_하자
    }
}

// incoming event handler
impl NetworkBehaviourEventProcess<FloodsubEvent> for AppBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        if let FloodsubEvent::Message(msg) = event {
            if let Ok(응답) = serde_json::from_slice::<ChainResponse>(&msg.data) {
                if 응답.receiver == PEER_ID.to_string() {
                    info!("Response from {}:", msg.source);
                    응답.블록들.iter().for_each(|r| info!("{:?}", r));

                    self.app.블록들 = self.app.체인_선택_함수(self.app.블록들.clone(), 응답.블록들);
                }
            } else if let Ok(응답) = serde_json::from_slice::<로칼_체인_요청_구조체>(&msg.data) {
                info!("sending 로칼 chain to {}", msg.source.to_string());
                let peer_id = 응답.출처_peer_id;
                if PEER_ID.to_string() == peer_id {
                    if let Err(e) = self.반응_송신자.send(ChainResponse {
                        블록들: self.app.블록들.clone(),
                        receiver: msg.source.to_string(),
                    }) {
                        error!("error sending response via channel, {}", e);
                    }
                }
            } else if let Ok(block) = serde_json::from_slice::<블록>(&msg.data) {
                info!("received new block from {}", msg.source.to_string());
                self.app.블록_추가시도_함수(block);
            }
        }
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for AppBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

pub fn peer_목록_얻기(swarm: &Swarm<AppBehaviour>) -> Vec<String> {
    info!("발견된 Peer:");
    let nodes = swarm.behaviour().mdns.discovered_nodes();//네트워크에서 찾은 노드 목록을 nodes 변수에 할당
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }
    unique_peers.iter().map(|p| p.to_string()).collect()
}

pub fn handle_print_peers(swarm: &Swarm<AppBehaviour>) {
    let peers = peer_목록_얻기(swarm);
    peers.iter().for_each(|p| info!("{}", p));
}

pub fn 체인_출력_처리_함수(swarm: &Swarm<AppBehaviour>) {
    info!("Local Blockchain:");
    let pretty_json =
        serde_json::to_string_pretty(&swarm.behaviour().app.블록들).expect("can jsonify 블록들");
    info!("{}", pretty_json);
}

pub fn 새_블록_생성_처리_함수(cmd: &str, swarm: &mut Swarm<AppBehaviour>) {
    if let Some(데이터) = cmd.strip_prefix("new block") {
        let 처리_하자 = swarm.behaviour_mut();
        let 마지막_블록 = 처리_하자
            .app
            .블록들
            .last()
            .expect("there is at least one block");
        let block = 블록::new(
            마지막_블록.id + 1,
            마지막_블록.해시.clone(),
            데이터.to_owned(),
        );
        let json = serde_json::to_string(&block).expect("can jsonify request");
        처리_하자.app.블록들.push(block);
        info!("broadcasting new block");
        처리_하자
            .floodsub
            .publish(BLOCK_TOPIC.clone(), json.as_bytes());
    }
}
