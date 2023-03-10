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

// Self를 Self 대신 구조체 이름 블록으로 변경하여 impl 블록 구현체의 함수 시그니처에 사용할 수 있습니다. 이를 통해 코드를 보다 가독성 높은 방향으로 개선할 수 있습니다.

impl 블록 {
    pub fn new(id: u64, 이전_해시: String, 데이터: String) -> 블록 {
        let now = Utc::now();
        let (논스, 해시) = 블록_채굴(id, now.timestamp(), &이전_해시, &데이터);
        블록 {
            id,
            해시,
            타임스탬프: now.timestamp(),
            이전_해시,
            데이터,
            논스,
        }
    }
}

// impl 블록 {
//     pub fn new(id: u64, 이전_해시: String, 데이터: String) -> Self {
//         let now = Utc::now();
//         let (논스, 해시) = 블록_채굴(id, now.timestamp(), &이전_해시, &데이터);
//         Self {
//             id,
//             해시,
//             타임스탬프: now.timestamp(),
//             이전_해시,
//             데이터,
//             논스,
//         }
//     }
// }

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
            .expect("로컬 소켓을 얻을 수 있음"),
    )
    .expect("swarm을 시작할 수 있음");

    spawn(async move {
        sleep(Duration::from_secs(1)).await;
        info!("초기 이벤트 전송 중");
        초기_송신자.send(true).expect("초기 이벤트 전송 가능");
    });

    loop {
        let evt = {
            select! {
                라인 = stdin.next_line() => Some(peer2peer::이벤트_유형_열거형_데이타::Input(라인.expect("라인을 얻을 수 있음").expect("stdin에서 라인을 읽을 수 있음"))),
                response = 반응_수신.recv() => {
                    Some(peer2peer::이벤트_유형_열거형_데이타::로컬_체인_반응(response.expect("반응이 존재함")))
                },
                _init = 초기_수신.recv() => {
                    Some(peer2peer::이벤트_유형_열거형_데이타::Init)
                }
                event = swarm.select_next_some() => {
                    info!("처리되지 않은 Swarm Event: {:?}", event);
                    None
                },
            }
        };

        // RUST_LOG 환경변수 값이 "info" 이상일 때 "명령어 안내" 출력
        if let Ok(level_filter) = std::env::var("RUST_LOG") {
        if level_filter == "info" || level_filter == "debug" || level_filter == "trace" {
            println!("명령어 : show peer (피어 목록보기),show chain (블록 체인보기), new block(블록 생성)");
        }
    }
        if let Some(event) = evt {
            match event {
                peer2peer::이벤트_유형_열거형_데이타::Init => {
                    let peers = peer2peer::peer_목록_얻기(&swarm);
                    swarm.behaviour_mut().app.제네시스_함수();

                    info!("연결된 노드들: {}", peers.len());
                    if !peers.is_empty() {
                        let req = peer2peer::로칼_체인_요청_구조체 {
                            출처_peer_id: peers
                                .iter()
                                .last()
                                .expect("최소 하나의 피어")
                                .to_string(),
                        };

                        let json = serde_json::to_string(&req).expect("요청을 json화 할 수 있음");
                        swarm
                            .behaviour_mut()
                            .floodsub
                            .publish(peer2peer::CHAIN_TOPIC.clone(), json.as_bytes());
                    }
                }
                peer2peer::이벤트_유형_열거형_데이타::로컬_체인_반응(응답) => {
                    let json = serde_json::to_string(&응답).expect("반응을 json 화 할 수 있음");
                    swarm
                        .behaviour_mut()
                        .floodsub
                        .publish(peer2peer::CHAIN_TOPIC.clone(), json.as_bytes());
                }
                peer2peer::이벤트_유형_열거형_데이타::Input(라인) => match 라인.as_str() {
                    "show peer" => peer2peer::handle_print_peers(&swarm),
                    cmd if cmd.starts_with("show chain") => peer2peer::체인_출력_처리_함수(&swarm),
                    cmd if cmd.starts_with("new block") => peer2peer::새_블록_생성_처리_함수(cmd, &mut swarm),
                    _ => error!("모르는 명령"),
                },
            }
        }
    }
}
