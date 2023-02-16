use serde_json::json;
use chrono::prelude::*;
use libp2p::{
    core::upgrade,
    futures::StreamExt,
    mplex,
    noise::{Keypair, NoiseConfig, X25519Spec},
    swarm::{Swarm, SwarmBuilder},
    tcp::TokioTcpConfig,
    Transport,
};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    select, spawn,
    sync::mpsc,
    time::sleep,
};

const 난이도: &str = "00";

mod peer2peer;

pub struct 앱 {
    pub 블록들: Vec<블록>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct 블록 {
    pub id: u64,
    pub 해시: String,
    pub 이전_해시: String,
    pub 타임스탬프: i64,
    pub 데이터: String,
    pub 논스: u64,
}

impl 블록 {
    pub fn new(id: u64, 이전_해시: String, 데이터: String) -> Self {
        let now = Utc::now();
        let (논스, 해시) = 블록_채굴(id, now.timestamp(), &이전_해시, &데이터);
        Self {
            id,
            해시,
            타임스탬프: now.timestamp(),
            이전_해시,
            데이터,
            논스,
        }
    }
}

fn 해쉬_계산(id: u64, 타임스탬프: i64, 이전_해시: &str, 데이터: &str, 논스: u64) -> Vec<u8> {
    let 데이터 = serde_json::to_value(&json!({
        "id": id,
        "이전_해시": 이전_해시,
        "데이터": 데이터,
        "타임스탬프": 타임스탬프,
        "논스": 논스
    })).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(데이터.to_string().as_bytes());
    hasher.finalize().to_vec()
}

// 리눅스에서 오류나는 코드
// fn 블록_채굴(id: u64, 타임스탬프: i64, 이전_해시: &str, 데이터: &str) -> (u64, String) {
//     info!("블록 채굴...");
//     let mut 논스 = 0;

//     for _ in 0..std::usize::MAX {
//         if 논스 % 100000 == 0 {
//             info!("논스: {}", 논스);
//         }
//         let 해시 = 해쉬_계산(id, 타임스탬프, 이전_해시, 데이터, 논스);
//         let 이진_해쉬 = 해쉬_이진수_표현(&해시);
//         if 이진_해쉬.starts_with(난이도) {
//             info!(
//                 "성공! 논스: {}, 해시: {}, binary 해시: {}",
//                 논스,
//                 hex::encode(&해시),
//                 이진_해쉬
//             );
//             return (논스, hex::encode(해시));
//         }
//         논스 += 1;
//     }
//     (0, "".to_string())
// } 

fn 블록_채굴(id: u64, 타임스탬프: i64, 이전_해시: &str, 데이터: &str) -> (u64, String) {
    info!("블록 채굴...");
    let mut 논스 = 0;

    loop {
        if 논스 % 100000 == 0 {
            info!("논스: {}", 논스);
        }
        let 해시 = 해시_계산(id, 타임스탬프, 이전_해시, 데이터, 논스);
        let 이진_해시 = 해쉬_이진수_표현(&해시);
        if 이진_해시.starts_with(난이도) {
            info!(
                "성공! 논스: {}, 해시: {}, binary 해시: {}",
                논스,
                hex::encode(&해시),
                이진_해시
            );
            return (논스, hex::encode(해시));
        }
        논스 += 1;
    }
}
//변수만 아니라 코드도 바꾼 부분
fn 해쉬_이진수_표현(해시: &[u8]) -> String {
    해시.iter().map(|z| format!("{:b}", z)).collect::<String>()
}

impl 앱 {
    fn new() -> Self {
        Self { 블록들: vec![] }
    }

    fn 제네시스_함수(&mut self) {
        let 제네시스_블록_변수 = 블록 {
            id: 0,
            타임스탬프: Utc::now().timestamp(),
            이전_해시: String::from("제네시스"),
            데이터: String::from("제네시스!!"),
            논스: 2836,
            해시: "111111111111111111111111111111111111111111111111".to_string(),
        };
        self.블록들.push(제네시스_블록_변수);
    }

    //
    fn 블록_추가시도_함수(&mut self, block: 블록) {
        let 마지막_블록 = self.블록들.last().expect("적어도 하나의 블록이 존재");
        match self.블록_유효성확인_함수(&block, 마지막_블록) {
        true => self.블록들.push(block),
        false => error!("블록 추가 불가 - 유효하지 않음"),
        }
    }

    fn 블록_유효성확인_함수(&self, block: &블록, previous_block: &블록) -> bool {
        if block.이전_해시 != previous_block.해시 {
            warn!("block with id: {} has wrong previous 해시", block.id);
            return false;
        } else if !해쉬_이진수_표현(
            &hex::decode(&block.해시).expect("can decode from hex"),
        )
        .starts_with(난이도)
        {
            warn!("block with id: {} has invalid difficulty", block.id);
            return false;
        } else if block.id != previous_block.id + 1 {
            warn!(
                "block with id: {} is not the next block after the latest: {}",
                block.id, previous_block.id
            );
            return false;
        } else if hex::encode(해시_계산(
            block.id,
            block.타임스탬프,
            &block.이전_해시,
            &block.데이터,
            block.논스,
        )) != block.해시
        {
            warn!("block with id: {} has invalid 해시", block.id);
            return false;
        }
        true
    }

    fn is_chain_valid(&self, chain: &[블록]) -> bool {
        for i in 0..chain.len() {
            if i == 0 {
                continue;
            }
            let first = chain.get(i - 1).expect("has to exist");
            let second = chain.get(i).expect("has to exist");
            if !self.블록_유효성확인_함수(second, first) {
                return false;
            }
        }
        true
    }

    // We always choose the longest valid chain
    fn choose_chain(&mut self, local: Vec<블록>, remote: Vec<블록>) -> Vec<블록> {
        let is_local_valid = self.is_chain_valid(&local);
        let is_remote_valid = self.is_chain_valid(&remote);

        if is_local_valid && is_remote_valid {
            if local.len() >= remote.len() {
                local
            } else {
                remote
            }
        } else if is_remote_valid && !is_local_valid {
            remote
        } else if !is_remote_valid && is_local_valid {
            local
        } else {
            panic!("local and remote chains are both invalid");
        }
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    info!("Peer Id: {}", peer2peer::PEER_ID.clone());
    let (response_sender, mut response_rcv) = mpsc::unbounded_channel();
    let (init_sender, mut init_rcv) = mpsc::unbounded_channel();

    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&peer2peer::KEYS)
        .expect("can create auth keys");

    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    let behaviour = peer2peer::AppBehaviour::new(앱::new(), response_sender, init_sender.clone()).await;

    let mut swarm = SwarmBuilder::new(transp, behaviour, *peer2peer::PEER_ID)
        .executor(Box::new(|fut| {
            spawn(fut);
        }))
        .build();

    let mut stdin = BufReader::new(stdin()).lines();

    Swarm::listen_on(
        &mut swarm,
        "/ip4/0.0.0.0/tcp/0"
            .parse()
            .expect("can get a local socket"),
    )
    .expect("swarm can be started");

    spawn(async move {
        sleep(Duration::from_secs(1)).await;
        info!("sending init event");
        init_sender.send(true).expect("can send init event");
    });

    loop {
        let evt = {
            select! {
                line = stdin.next_line() => Some(peer2peer::EventType::Input(line.expect("can get line").expect("can read line from stdin"))),
                response = response_rcv.recv() => {
                    Some(peer2peer::EventType::LocalChainResponse(response.expect("response exists")))
                },
                _init = init_rcv.recv() => {
                    Some(peer2peer::EventType::Init)
                }
                event = swarm.select_next_some() => {
                    info!("Unhandled Swarm Event: {:?}", event);
                    None
                },
            }
        };

        if let Some(event) = evt {
            match event {
                peer2peer::EventType::Init => {
                    let peers = peer2peer::get_list_peers(&swarm);
                    swarm.behaviour_mut().app.제네시스_함수();

                    info!("connected nodes: {}", peers.len());
                    if !peers.is_empty() {
                        let req = peer2peer::LocalChainRequest {
                            from_peer_id: peers
                                .iter()
                                .last()
                                .expect("at least one peer")
                                .to_string(),
                        };

                        let json = serde_json::to_string(&req).expect("can jsonify request");
                        swarm
                            .behaviour_mut()
                            .floodsub
                            .publish(peer2peer::CHAIN_TOPIC.clone(), json.as_bytes());
                    }
                }
                peer2peer::EventType::LocalChainResponse(resp) => {
                    let json = serde_json::to_string(&resp).expect("can jsonify response");
                    swarm
                        .behaviour_mut()
                        .floodsub
                        .publish(peer2peer::CHAIN_TOPIC.clone(), json.as_bytes());
                }
                peer2peer::EventType::Input(line) => match line.as_str() {
                    "ls p" => peer2peer::handle_print_peers(&swarm),
                    cmd if cmd.starts_with("ls c") => peer2peer::handle_print_chain(&swarm),
                    cmd if cmd.starts_with("create b") => peer2peer::handle_create_block(cmd, &mut swarm),
                    _ => error!("unknown command"),
                },
            }
        }
    }
}
