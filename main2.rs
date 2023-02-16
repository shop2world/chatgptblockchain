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
        let 해시 = 해쉬_계산(id, 타임스탬프, 이전_해시, 데이터, 논스);
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
            warn!("id: {} 인 블록은 잘못된 이전 해시를 가짐", block.id);
            return false;
        } else if !해쉬_이진수_표현(
            &hex::decode(&block.해시).expect("16진수로부터 디코딩할 수 있음."),
        )
        .starts_with(난이도)
        {
            warn!("id: {} 인 블록의 난이도가 잘못되었습니다.", block.id);
            return false;
        } else if block.id != previous_block.id + 1 {
            warn!(
                "id: {} 를 가진 블록은 유효하지 않은 난이도를 가지고 있습니다.: {}",
                block.id, previous_block.id
            );
            return false;
        } else if hex::encode(해쉬_계산(
            block.id,
            block.타임스탬프,
            &block.이전_해시,
            &block.데이터,
            block.논스,
        )) != block.해시
        {
            warn!("블록 id: {} 의 해시가 올바르지 않습니다", block.id);
            return false;
        }
        true
    }

    fn 체인_유효성_확인_함수(&self, chain: &[블록]) -> bool {
        for i in 0..chain.len() {
            if i == 0 {
                continue;
            }
            let 첫번째 = chain.get(i - 1).expect("존재해야 합니다");
            let 두번째 = chain.get(i).expect("존재해야 합니다");
            if !self.블록_유효성확인_함수(두번째, 첫번째) {
                return false;
            }
        }
        true
    }

    // 긴체인이 유효한 체인
    fn 체인_선택_함수(&mut self, 로칼: Vec<블록>, 외부: Vec<블록>) -> Vec<블록> {
        let 로칼_유효 = self.체인_유효성_확인_함수(&로칼);
        let 외부_유효 = self.체인_유효성_확인_함수(&외부);

        if 로칼_유효 && 외부_유효 {
            if 로칼.len() >= 외부.len() {
                로칼
            } else {
                외부
            }
        } else if 외부_유효 && !로칼_유효 {
            외부
        } else if !외부_유효 && 로칼_유효 {
            로칼
        } else {
            panic!("로칼과 외부 체인 모두 유효하지 않습니다");
        }
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    info!("Peer Id: {}", peer2peer::PEER_ID.clone());
    let (반응_송신자, mut 반응_수신) = mpsc::unbounded_channel();
    let (초기_송신자, mut 초기_수신) = mpsc::unbounded_channel();

    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&peer2peer::KEYS)
        .expect("인증 키를 생성할 수 있습니다.");

    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    let 처리_하자 = peer2peer::AppBehaviour::new(앱::new(), 반응_송신자, 초기_송신자.clone()).await;

    let mut swarm = SwarmBuilder::new(transp, 처리_하자, *peer2peer::PEER_ID)
        .executor(Box::new(|fut| {
            spawn(fut);
        }))
        .build();

    let mut stdin = BufReader::new(stdin()).lines();

    Swarm::listen_on(
        &mut swarm,
        "/ip4/0.0.0.0/tcp/0"
            .parse()
            .expect("can get a 로칼 socket"),
    )
    .expect("swarm can be started");

    spawn(async move {
        sleep(Duration::from_secs(1)).await;
        info!("sending init event");
        초기_송신자.send(true).expect("can send init event");
    });

    loop {
        let evt = {
            select! {
                line = stdin.next_line() => Some(peer2peer::EventType::Input(line.expect("can get line").expect("can read line from stdin"))),
                response = 반응_수신.recv() => {
                    Some(peer2peer::EventType::LocalChainResponse(response.expect("response exists")))
                },
                _init = 초기_수신.recv() => {
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
