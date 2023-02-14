use serde_json::json;
// chrono::prelude::* 모듈은 Rust의 라이브러리인 chrono 라이브러리에서 제공하는 
// 시간 관련 기능을 포함하고 있습니다. 
// prelude 는 Rust의 표준 라이브러리에서 제공하는 미리 준비된 기능 집합을 의미하며, 
// 이를 사용하여 날짜와 시간 관련 기능을 쉽게 사용할 수 있습니다. 
// * 기호는 모듈의 모든 기능을 가져오는 것을 의미합니다.
use chrono::prelude::*;
// 아래의 "libp2p" 모듈은 플러그인 기반 P2P 네트워크 애플리케이션을 개발하는 데 필요한 도구를 제공하는 Rust 라이브러리입니다. 여기서 포함된 도구로는:

// "upgrade" 모듈: 네트워크 프로토콜을 업그레이드하는 기능을 제공합니다.
// "StreamExt" 모듈: Futures Stream 객체에서 편리한 메서드를 제공합니다.
// "mplex" 모듈: 멀티플렉스 프로토콜을 제공합니다.
// "NoiseConfig" 모듈: 노이즈 프로토콜을 정의하는 기능을 제공합니다.
// "X25519Spec" 모듈: X25519 키 스펙을 정의합니다.
// X25519 key specification은 공개 키 암호화 알고리즘인 X25519의 
// 키 생성, 관리, 교환에 관련된 규약을 말합니다. 
// X25519는 고속의 공개 키 암호화 알고리즘으로, 입력의 길이가 짧고, 계산 비용이 적어 
// 클라이언트/서버 간의 대용량 통신에 적합합니다.
// "Swarm" 모듈: Swarm 구조체를 정의합니다.
// "SwarmBuilder" 모듈: Swarm 구조체를 빌드하는 데 필요한 도구를 제공합니다.
// "TokioTcpConfig" 모듈: Tokio 기반 TCP 설정을 정의합니다.
// "Transport" 모듈: 네트워크 전송 기능을 제공합니다.
use libp2p::{
    core::upgrade,
    futures::StreamExt,
    mplex,
    noise::{Keypair, NoiseConfig, X25519Spec},
    swarm::{Swarm, SwarmBuilder},
    tcp::TokioTcpConfig,
    Transport,
};
// "log" 모듈은 Rust 프로그래밍 언어에서 로그 출력 기능을 제공하는 모듈입니다. "error", "info", "warn"은 로깅 레벨로, 에러, 정보, 경고 메시지를 출력하는 기능을 제공합니다. 이 모듈은 애플리케이션의 동작상태를 디버깅하기 위해 사용될 수 있습니다.
use log::{error, info, warn};
//"serde" 모듈은 객체를 직렬화(Serialization)하여 바이트 데이터로 변환하고, 바이트 데이터를 다시 객체로 복원하는 기능(Deserialization)을 제공하는 모듈입니다.
use serde::{Deserialize, Serialize};
// "sha2::{Digest, Sha256}" 모듈은 sha2 패키지에서 Digest 그리고 Sha256 타입을 
// 사용하겠다는 뜻입니다. SHA-256은 256비트의 메시지 다이제스트 해시 함수입니다. 
// 이 해시 함수를 사용하면 데이터의 무결성을 검증하고, 이를 통해 변조된 데이터를 감지할 수 있습니다.
use sha2::{Digest, Sha256};
// std::time::Duration 모듈은 Rust 표준 라이브러리에서 시간 관련 기능을 제공하는 모듈입니다. Duration 구조체를 사용하여 특정 기간을 나타낼 수 있습니다.
// 예를 들어, Duration::new(2, 0)은 2초를 나타냅니다. 또한, Duration 구조체에서 제공하는 다양한 메서드를 사용하여 초, 마이크로초, 나노초 등 단위의 시간을 더하거나 빼거나 비교할 수 있습니다.
use std::time::Duration;
// tokio 모듈은 비동기 I/O와 코루틴(coroutine)을 구현하는 라이브러리입니다.
// 코루틴(coroutine)은 , 여러 개의 실행 흐름(flow of execution)을 하나의 프로그램 내에서 동시에 수행할 수 있도록 해줍니다. 코루틴은 쓰레드와 비슷한 방식으로 동작하지만, 쓰레드의 많은 부하와 복잡성을 줄일 수 있는 장점이 있습니다.

// io 모듈은 표준 입력과 버퍼 리더 기능을 제공합니다.
// select 함수는 여러 개의 비동기 태스크 중 하나를 선택하는 기능을 제공합니다.
// spawn 함수는 새로운 비동기 태스크를 생성하는 기능을 제공합니다.
// mpsc 모듈은 여러 개의 태스크 간의 메시지 전송을 위한 기능을 제공합니다.
// sleep 함수는 지정된 시간동안 대기하는 기능을 제공합니다.
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



// "#[derive]"는 Rust 프로그래밍 언어에서의 매크로 어노테이션이다. "derive" 키워드는 Rust 컴파일러에게 특정 트레잇(trait)의 기본 구현을 생성하도록 지시한다. 예를 들어, "Serialize"라는 트레잇의 기본 구현을 생성하는 것이 가능하다. 만약, "Deserialize"라는 트레잇의 기본 구현을 생성하는 것이 가능하다면 그것도 가능하다.
// "Debug" 트레잇의 기본 구현을 생성하는 것도 가능하다. 이 트레잇은 객체의 디버깅 정보를 출력하는 기본 구현을 생성하도록 한다.
// "Clone" 트레잇의 기본 구현을 생성하는 것도 가능하다. 이 트레잇은 객체의 복제 기능을 생성하도록 한다

// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub struct 블록 {
//     pub id: u64,
//     pub 해시: String,
//     pub 이전_해시: String,
//     pub 타임스탬프: i64,
//     pub 데이터: String,
//     pub 논스: u64,
// }

// Rust 프로그래밍 언어에서 #[derive(...)] 매크로를 사용한 것입니다. derive 매크로는 구조체, 열거형 등의 타입에서 자동으로 특정 트레잇(trait)을 구현하는 것을 도와줍니다.

// 여기서 구현하는 트레잇은 다음과 같습니다.

// Serialize: 이 타입을 직렬화(serialize)할 수 있다는 것을 나타냅니다.
// Deserialize: 이 타입을 역직렬화(deserialize)할 수 있다는 것을 나타냅니다.
// Debug: 이 타입을 디버깅할 때 {:?} 포맷으로 출력할 수 있다는 것을 나타냅니다.
// Clone: 이 타입을 복제할 수 있다는 것을 나타냅니다.
// Eq: 이 타입이 같은지 비교할 수 있다는 것을 나타냅니다.
// PartialEq: 이 타입이 일부분 같은지 비교할 수 있다는 것을 나타냅니다.

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct 블록 {
    id: u64,
    타임스탬프: i64,
    데이터: String,
    이전_해시: String,
    해시: String,
    논스: u64,
    
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

// fn 해쉬_계산(id: u64, 타임스탬프: i64, 이전_해시: &str, 데이터: &str, 논스: u64) -> Vec<u8> {
//     let 데이터 = serde_json::json!({
//         "id": id,
//         "이전_해시": 이전_해시,
//         "데이터": 데이터,
//         "타임스탬프": 타임스탬프,
//         "논스": 논스
//     });
//     let mut hasher = Sha256::new();
//     hasher.update(데이터.to_string().as_bytes());
//     hasher.finalize().as_slice().to_owned()
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




// fn 블록_채굴(id: u64, 타임스탬프: i64, 이전_해시: &str, 데이터: &str) -> (u64, String) {
//     info!("블록 채굴...");
//     let mut 논스 = 0;

//     loop {
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
// }

fn 블록_채굴(id: u64, 타임스탬프: i64, 이전_해시: &str, 데이터: &str) -> (u64, String) {
    info!("블록 채굴...");
    let mut 논스 = 0;

    for _ in 0..std::usize::MAX {
        if 논스 % 100000 == 0 {
            info!("논스: {}", 논스);
        }
        let 해시 = 해쉬_계산(id, 타임스탬프, 이전_해시, 데이터, 논스);
        let 이진_해쉬 = 해쉬_이진수_표현(&해시);
        if 이진_해쉬.starts_with(난이도) {
            info!(
                "성공! 논스: {}, 해시: {}, binary 해시: {}",
                논스,
                hex::encode(&해시),
                이진_해쉬
            );
            return (논스, hex::encode(해시));
        }
        논스 += 1;
    }
    (0, "".to_string())
} 



// fn 해쉬_이진수_표현(해시: &[u8]) -> String {
//     let mut 결과: String = String::default();
//     for z in 해시 {
//         결과.push_str(&format!("{:b}", z));
//     }
//     결과
// }

// 챗 gpt 이 rust 코드는 해쉬_이진수_표현 함수이다. 이 함수는 해시 바이트 배열을 받아, 각 바이트를 2진수 문자열로 변환하여 하나의 연속된 문자열로 만든 뒤 반환한다.

// 함수 내에서, 해시 배열의 각 바이트에 대해 iter 메소드를 호출하여 바이트의 반복자(iterator)를 생성한다. 그리고 각 반복자에 대해 map 메소드를 호출하여, 각 바이트를 2진수 문자열 "{:b}" 로 포매팅한다. collect 메소드를 호출하여 만들어진 모든 2진수 문자열을 하나의 연속된 문자열 String 으로 모은다.

// 최종적으로, 함수는 만들어진 문자열 String을 반환한다.

fn 해쉬_이진수_표현(해시: &[u8]) -> String {
    해시.iter().map(|z| format!("{:b}", z)).collect::<String>()
}


impl 앱 {
    // fn new() -> Self {
    //     Self { 블록들: vec![] }
    // }
    fn new() -> 앱 {
        앱 { 블록들: vec![] }
    }

    fn 제네시스_함수(&mut self) {
        let 제네시스블록_변수 = 블록 {
            id: 0,
            타임스탬프: Utc::now().timestamp(),//Utc::now() 함수는 Coordinated Universal Time (UTC) 기준으로 현재 시각을 반환
            이전_해시: String::from("제네시스"),
            데이터: String::from("제네시스!"),
            논스: 2836,
            해시: "1010f816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c43".to_string(),
        };
        self.블록들.push(제네시스블록_변수);
    }
    

    // fn 블록_추가시도_함수(&mut self, block: 블록) {
    //     let 마지막_블록 = self.블록들.last().expect("적어도 하나의 블록이 존재");
    //     if self.블록_유효성확인_함수(&block, 마지막_블록) {
    //         self.블록들.push(block);
    //     } else {
    //         error!("블록 추가 불가 - 유효하지 않음");
    //     }
    // }

    fn 블록_추가시도_함수(&mut self, block: 블록) {
        let 마지막_블록 = self.블록들.last().expect("적어도 하나의 블록이 존재");
        match self.블록_유효성확인_함수(&block, 마지막_블록) {
        true => self.블록들.push(block),
        false => error!("블록 추가 불가 - 유효하지 않음"),
        }
    }


    

    // fn 블록_유효성확인_함수(&self, block: &블록, previous_block: &블록) -> bool {
    //     if block.이전_해시 != previous_block.해시 {
    //         warn!("id: {} 인 블록은 잘못된 이전 해시를 가짐", block.id);
    //         return false;
    //     } else if !해쉬_이진수_표현(
    //         &hex::decode(&block.해시).expect("16진수로부터 디코딩할 수 있음."),
    //     )
    //     .starts_with(난이도)
    //     {
    //         warn!("id: {} 인 블록의 난이도가 잘못되었습니다.", block.id);
    //         return false;
    //     } else if block.id != previous_block.id + 1 {
    //         warn!(
    //             "id: {} 를 가진 블록은 유효하지 않은 난이도를 가지고 있습니다.: {}",
    //             block.id, previous_block.id
    //         );
    //         return false;
    //     } else if hex::encode(해쉬_계산(
    //         block.id,
    //         block.타임스탬프,
    //         &block.이전_해시,
    //         &block.데이터,
    //         block.논스,
    //     )) != block.해시
    //     {
    //         warn!("블록 id: {} 의 해시가 올바르지 않습니다", block.id);
    //         return false;
    //     }
    //     true
    // }

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

// 아래 Rust 코드는 '체인_유효성_확인_함수' 라는 함수를 정의하고 있습니다. 이 함수는 블록 체인에 대한 유효성을 확인하는 데 사용됩니다.

// 함수의 매개 변수 chain은 블록 체인을 나타내는 배열입니다. 함수는 루프를 실행하여 각 블록에 대해 블록 유효성을 확인합니다.

// 만약 유효성 확인에서 false 값이 반환된다면, 함수는 바로 false 값을 반환하여 유효하지 않은 것으로 간주합니다. 그렇지 않으면, 함수는 true 값을 반환하여 유효한 것으로 간주합니다.

    // fn 체인_유효성_확인_함수(&self, chain: &[블록]) -> bool {
    //     for i in 0..chain.len() {
    //         if i == 0 {
    //             continue;
    //         }
    //         let 첫번째 = chain.get(i - 1).expect("존재해야 합니다");
    //         let 두번째 = chain.get(i).expect("존재해야 합니다");
    //         if !self.블록_유효성확인_함수(두번째, 첫번째) {
    //             return false;
    //         }
    //     }
    //     true
    // }

    fn 체인_유효성_확인_함수(&self, chain: &[블록]) -> bool {
        let mut i = 0;
        while i < chain.len() {
            if i == 0 {
            i += 1;
            continue;
            }
            let 첫번째 = chain.get(i - 1).expect("존재해야 합니다");
            let 두번째 = chain.get(i).expect("존재해야 합니다");
            if !self.블록_유효성확인_함수(두번째, 첫번째) {
                return false;
            }
            i += 1;
        }
        true
    }

    // 우리는 항상 가장 긴 유효한 체인을 선택합니다
    fn 체인_선택_함수(&mut self, 로칼: Vec<블록>, 외부: Vec<블록>) -> Vec<블록> {
        let 로칼_유효 = self.체인_유효성_확인_함수(&로칼);
        let 외부remote_유효 = self.체인_유효성_확인_함수(&외부);

        if 로칼_유효 && 외부remote_유효 {
            if 로칼.len() >= 외부.len() {
                로칼
            } else {
                외부
            }
        } else if 외부remote_유효 && !로칼_유효 {
            외부
        } else if !외부remote_유효 && 로칼_유효 {
            로칼
        } else {
            panic!("로칼과 외부 체인 모두 유효하지 않습니다");
        }
    }
}

#[tokio::main]
async fn main() {
    // pretty_env_logger 는 Rust 프로그래밍 언어에서의 라이브러리 이름입니다. 이 라이브러리는 Rust 프로젝트에서 환경 변수를 통해 로깅을 설정할 수 있도록 도와주는 로깅 라이브러리입니다.
    pretty_env_logger::init();

    info!("Peer Id: {}", peer2peer::피어_아이디.clone());
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

    let 처리_하자 = peer2peer::앱동작_구조체::new(앱::new(), 반응_송신자, 초기_송신자.clone()).await;

    let mut swarm = SwarmBuilder::new(transp, 처리_하자, *peer2peer::피어_아이디)
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
        let 이벤트 = {
            select! {
                //next_line()은 Rust 표준 라이브러리의 io 모듈에서 제공하는 함수. 사용자로부터 한 줄의 입력을 받기 위해서 사용
                라인 = stdin.next_line() => Some(peer2peer::이벤트_유형_열거형_데이타::Input(라인.expect("라인을 얻을 수 있음").expect("stdin에서 라인을 읽을 수 있음"))), //expect는 Rust 표준 라이브러리에 포함된 함수이다. expect 함수는 Result 값이 Ok인지 확인하고, 결과가 Ok이면 그 안에 있는 값을 반환한다. 그렇지 않으면 expect 함수는 주어진 문자열을 메시지로 가지는 Panic을 발생시킨다.
                반응 = 반응_수신.recv() => {
                    Some(peer2peer::이벤트_유형_열거형_데이타::로컬_체인_반응(반응.expect("반응이 존재함")))
                },
                _초기 = 초기_수신.recv() => {
                    Some(peer2peer::이벤트_유형_열거형_데이타::Init)
                }
                event = swarm.select_next_some() => {
                    info!("처리되지 않은 Swarm Event: {:?}", event);
                    None
                },
            }
        };

        if let Some(event) = 이벤트 {
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
                                .expect("at least one peer")
                                .to_string(),
                        };

                        let json = serde_json::to_string(&req).expect("요청을 json화 할 수 있음");
                        swarm
                            .behaviour_mut()
                            .floodsub
                            .publish(peer2peer::체인_토픽.clone(), json.as_bytes());
                    }
                }
                peer2peer::이벤트_유형_열거형_데이타::로컬_체인_반응(응답) => {
                    let json = serde_json::to_string(&응답).expect("반응을 json 화 할 수 있음");
                    swarm
                        .behaviour_mut()
                        .floodsub
                        .publish(peer2peer::체인_토픽.clone(), json.as_bytes());
                }
                peer2peer::이벤트_유형_열거형_데이타::Input(라인) => match 라인.as_str() {
                    "ls p" => peer2peer::연결된_peer_출력_함수(&swarm),
                    터미날 if 터미날.starts_with("ls c") => peer2peer::체인_출력_처리_함수(&swarm),
                    터미날 if 터미날.starts_with("create b") => peer2peer::새_블록_생성_처리_함수(터미날, &mut swarm),
                    _ => error!("모르는 명령"),
                },
            }
        }
    }

    
}




// 참고 자료
// 메소드와 함수의 차이

// 메소드와 함수의 차이는 다음과 같습니다.

// 정의 방법: 메소드는 구조체, 클래스, 트레이트 등에서 정의되는 것이고, 함수는 모듈 레벨에서 정의되는 것입니다.

// 접근성: 메소드는 객체 안에서만 호출할 수 있지만, 함수는 모듈 레벨에서도 호출할 수 있습니다.

// 바인딩: 메소드는 객체에 바인딩되어 호출되는데, 그 객체의 속성을 사용할 수 있습니다. 함수는 객체에 바인딩되지 않아 객체의 속성을 사용할 수 없습니다.

// 구조: 메소드는 구조체, 클래스, 트레이트에 속한 것이고, 함수는 모듈 레벨에 속한 것입니다.


// 직렬화와 역직렬화

// 직렬화 (Serialization)은 객체 또는 데이터 구조를 바이트 스트림 또는 기타 직렬화 형식으로 변환하는 것을 의미합니다. 역직렬화 (Deserialization)은 바이트 스트림 또는 기타 직렬화 형식을 객체 또는 데이터 구조로 복원하는 것을 의미합니다.

// 예를 들어, Rust의 Serde 패키지를 사용하여 JSON 형식으로 직렬화하는 경우, 객체 또는 데이터 구조를 JSON 형식의 문자열로 변환하고 역직렬화하면 JSON 문자열을 객체 또는 데이터 구조로 복원할 수 있습니다.

// use serde::{Serialize, Deserialize};

// #[derive(Serialize, Deserialize, Debug)]
// struct Person {
//     name: String,
//     age: u8,
// }

// fn main() {
//     let person = Person {
//         name: "John Doe".to_owned(),
//         age: 30,
//     };

//     // JSON 직렬화
//     let serialized = serde_json::to_string(&person).unwrap();
//     println!("Serialized: {}", serialized);

//     // JSON 역직렬화
//     let deserialized: Person = serde_json::from_str(&serialized).unwrap();
//     println!("Deserialized: {:?}", deserialized);
// }
// 결과

// Serialized: {"name":"John Doe","age":30}
// Deserialized: Person { name: "John Doe", age: 30 }

// 변환되는 내용을 이해하기 위해 예시를 더 들면 다음과 같습니다.

// 직렬화(Serialization)은 객체를 바이트 스트림 형태의 데이터로 변환하는 것을 말합니다. 객체의 내용이 여러 개의 프로퍼티로 구성되어 있고, 이 프로퍼티들은 바이트 스트림으로 변환할 수 있는 형태로 저장됩니다.

// 역직렬화(Deserialization)은 바이트 스트림 형태의 데이터를 객체 형태로 변환하는 것을 말합니다. 바이트 스트림에 저장된 데이터를 프로퍼티 형태로 구성하고, 객체로 변환할 수 있는 형태로 만듭니다.

// 예를 들어, 다음과 같은 객체가 있다고 가정합니다.

// #[derive(Serialize, Deserialize, Debug)]
// struct Person {
//     name: String,
//     age: u32,
// }

// let person = Person {
//     name: "John Doe".to_owned(),
//     age: 30,
// };

// 이 객체를 직렬화하면 다음과 같은 바이트 스트림이 만들어집니다.
// {"name":"John Doe","age":30}

// 이 바이트 스트림을 역직렬화하면 다시 Person 객체가 만들어집니다.

