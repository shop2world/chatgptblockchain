use super::{앱, 블록};
use libp2p::{
    NetworkBehaviour,
    identity,
    mdns::{Mdns, MdnsEvent},
    swarm::{
        NetworkBehaviourEventProcess, Swarm,
    },
    PeerId,
    floodsub::{Floodsub, FloodsubEvent, Topic},
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
pub struct 체인_반응_구조체 {
    pub 블록들: Vec<블록>,
    pub 수신자: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct 로칼_체인_요청_구조체 {
    pub 출처_peer_id: String,
}

pub enum 이벤트_유형_열거형_데이타 {
    로컬_체인_반응(체인_반응_구조체),
    Input(String),
    Init,
}

#[derive(NetworkBehaviour)]
pub struct 앱동작_구조체 {
    pub floodsub: Floodsub,
    pub mdns: Mdns,
    #[behaviour(ignore)]//컴파일러에게 해당 코드를 무시하도록 알려주는 것입니다. "behaviour"은 특정 기능을 정의하는 Rust 플러그인이고, "ignore"는 그 플러그인에서 정의한 기능 중 하나입니다.
    pub 반응_송신자: mpsc::UnboundedSender<체인_반응_구조체>,
    #[behaviour(ignore)]
    pub 초기_송신자: mpsc::UnboundedSender<bool>,
    #[behaviour(ignore)]
    pub app: 앱,
}

impl 앱동작_구조체 {
    pub async fn new(
        app: 앱,
        반응_송신자: mpsc::UnboundedSender<체인_반응_구조체>,
        초기_송신자: mpsc::UnboundedSender<bool>,
    ) -> Self {
        let mut behaviour = Self {
            app,
            floodsub: Floodsub::new(*PEER_ID),
            mdns: Mdns::new(Default::default())
                .await
                .expect("mdns를 만들 수 없음"),
            반응_송신자,
            초기_송신자,
        };
        // behaviour.floodsub.subscribe의 의미는 Floodsub 모듈의 subscribe 기능을 호출하고, 
        // 해당 기능을 통해 특정 토픽에 대한 구독을 수행하는 것을 의미합니다.이 구독은 libp2p 
        // 네트워크에 있는 다른 피어들에서 전달되는 해당 토픽에 대한 메시지를 받게 됩니다. 
        // Floodsub 모듈은 libp2p 피어 간에 메시지를 퍼뜨리는 (flood) 프로토콜을 제공하며, 
        // 이 구독 기능은 메시지를 받을 수 있는 방식을 제공합니다.

        behaviour.floodsub.subscribe(CHAIN_TOPIC.clone());
        behaviour.floodsub.subscribe(BLOCK_TOPIC.clone());

        behaviour
    }
}

// 수신 이벤트 핸들러

// NetworkBehaviourEventProcess 라는 모듈은 네트워크 동작 이벤트를 처리하는 모듈로서
// libp2p::swarm::NetworkBehaviourEventProcess 를 통해 가져와 사용
// Floodsub 모듈에서 inject_event 함수의 기능은 Floodsub 플러그인의 상태에 이벤트를 
// 주입하는 것입니다. 
// 이 함수는 Floodsub 플러그인의 상태를 업데이트하고, 이벤트에 대한 처리를 수행합니다. 
// 예를 들어, 새로운 구독에 대한 알림, 메시지 수신 등의 작업을 포함할 수 있습니다.

impl NetworkBehaviourEventProcess<FloodsubEvent> for 앱동작_구조체 {
    fn inject_event(&mut self, event: FloodsubEvent) {
        // FloodsubEvent::Message인 경우에는 if, else if로 처리하면서, 
        //나머지 경우에는 _로 처리하도록 했습니다. _는 모든 패턴에 매치되는 와일드카드 패턴입니다.
        match event {
            FloodsubEvent::Message(message) => {
                if let Ok(response) = serde_json::from_slice::<체인_반응_구조체>(&message.data) {
                    if response.수신자 == PEER_ID.to_string() {
                        info!("{}에서의 응답:", message.source);
                        response.블록들.iter().for_each(|r| info!("{:?}", r));
        
                        self.app.블록들 = self.app.체인_선택_함수(self.app.블록들.clone(), response.블록들);
                    }
                } else if let Ok(response) = serde_json::from_slice::<로칼_체인_요청_구조체>(&message.data) {
                    info!("로칼 체인을 {}에 보내는 중", message.source.to_string());
                    let peer_id = response.출처_peer_id;
                    if PEER_ID.to_string() == peer_id {
                        if let Err(e) = self.반응_송신자.send(체인_반응_구조체 {
                            블록들: self.app.블록들.clone(),
                            수신자: message.source.to_string(),
                        }) {
                            error!("채널로 반응을 보내는데 에러발생, {}", e);
                        }
                    }
                } else if let Ok(block) = serde_json::from_slice::<블록>(&message.data) {
                    info!("{} 에서 새로운 블록을 받음", message.source.to_string());
                    self.app.블록_추가시도_함수(block);
                }
            }
            _ => {}
        }
        
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for 앱동작_구조체 {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(발견된_노드_목록) => {
                for (peer, _addr) in 발견된_노드_목록 {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(만료된_노드_목록) => {
                for (peer, _addr) in 만료된_노드_목록 {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

pub fn peer_목록_얻기(swarm: &Swarm<앱동작_구조체>) -> Vec<String> {
    info!("발견된 피어들:");
    let nodes = swarm.behaviour().mdns.discovered_nodes();//네트워크에서 찾은 노드 목록을 nodes 변수에 할당
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }
    unique_peers.iter().map(|p| p.to_string()).collect()
}

//아래의 rust 함수는 Swarm<앱동작_구조체> 타입의 참조자 swarm을 인자로 받아, peer 목록을 얻어와 각 peer를 출력하는 함수입니다.
pub fn 연결된_peer_출력_함수(swarm: &Swarm<앱동작_구조체>) {
    let peers = peer_목록_얻기(swarm);
    peers.iter().for_each(|p| info!("{}", p));
}



pub fn 체인_출력_처리_함수(swarm: &Swarm<앱동작_구조체>) {
    info!("로컬 블록체인:");
    let 블록_json =
        serde_json::to_string_pretty(&swarm.behaviour().app.블록들).expect("블록들을 json으로 변환할 수 있음");
    info!("{}", 블록_json);
}
//
pub fn 새_블록_생성_처리_함수(cmd: &str, swarm: &mut Swarm<앱동작_구조체>) {
    match cmd.strip_prefix("new block") {
        Some(데이터) => {
            let behaviour = swarm.behaviour_mut();
            let 마지막_블록 = behaviour
                .app
                .블록들
                .last()
                .expect("적어도 하나의 블록이 있어야 합니다");
            let block = 블록::new(
                마지막_블록.id + 1,
                마지막_블록.해시.clone(),
                데이터.to_owned(),
            );

            let json = serde_json::to_string(&block).expect("블록들을 json으로 변환할 수 있음");
            behaviour.app.블록들.push(block);
            info!("새 블록을 broadcast 합니다");
            behaviour
                .floodsub
                .publish(BLOCK_TOPIC.clone(), json.as_bytes());
        },
        None => {},
    }
}






====================================================
// 챗GPT에게 요청: Rust 언어에서 super 키워드를 사용하여 현재 모듈의 상위 모듈에서 정의된 앱과 블록을 가져오는 것을 나타내 주세요. 즉, 상위 모듈에서 정의된 앱과 블록 구조체에 대한 레퍼런스를 현재 모듈에서 사용할 수 있도록 해주세요.
use super::{앱, 블록};

// 아래 모듈 libp2p 패키지의 모듈들을 불러오는 것을 설명합니다.

// identity 모듈은 PeerId 정보를 만드는 데 필요한 내용이 포함되어 있습니다.

// identity 모듈과 PeerId는 libp2p 네트워크에서 피어 식별자를 생성하는 기능을 제공하는데 

// 필요한 구성 요소입니다. 

// NetworkBehaviour 트레이트는 libp2p 네트워크 동작을 구현하는 방법을 정의하는데 

// 필요한 구성 요소입니다.

// floodsub 모듈은 비슷한 피어 간에 메시지를 퍼뜨리는 (flood) 프로토콜을 제공합니다. Floodsub 클래스와 FloodsubEvent 열거형, Topic 구조체가 포함되어 있습니다.

// mdns 모듈은 로컬 네트워크에 있는 다른 libp2p 피어를 찾기 위한 mdns 기능을 제공합니다. Mdns 클래스와 MdnsEvent 열거형이 포함되어 있습니다.

// swarm 모듈은 libp2p 피어 간 통신을 관리하는 Swarm 클래스와, Swarm 객체에서 발생하는 이벤트를 처리하는 NetworkBehaviourEventProcess 트레잇(traits)이 포함되어 있습니다.


use libp2p::{
    identity,PeerId,NetworkBehaviour,
    floodsub::{Floodsub, FloodsubEvent, Topic},
    mdns::{Mdns, MdnsEvent},
    swarm::{Swarm, NetworkBehaviourEventProcess},
};


use log::{error, info};
// 아래의 once_cell::sync::Lazy 모듈은 단 한 번만 초기화되는 static 변수를 정의할 때 사용하는 
// 라이브러리입니다. 이 모듈은 멀티스레드 환경에서도 안정적으로 동작하며, 
// static 변수의 초기화가 처음 호출될 때만 실행되는 것을 보장합니다. 
// 이 기능은 어플리케이션의 초기화 비용을 줄여주는 등의 여러 이점을 가집니다.
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
// 아래의 "std::collections::HashSet"은 Rust의 Standard Library(표준 라이브러리)에 포함된 
// HashSet 컬렉션을 가져오는 것을 나타냅니다.
// HashSet은 순서가 없고, 각 원소가 고유한 세트(Set) 구조를 제공합니다. 
// 원소의 순서는 중요하지 않고, 중복되는 값을 허용하지 않습니다. 
// 해시 테이블을 사용하여 원소의 유무를 효율적으로 확인하고 관리할 수 있습니다.
use std::collections::HashSet;
// 아래의 tokio::sync::mpsc 모듈은 tokio 패키지의 synchronization 부분에서 제공하는 
// multiple-producer single-consumer (mpsc) 큐와 관련된 기능을 제공합니다. 
// mpsc 큐는 여러 개의 producer가 하나의 consumer에게 데이터를 전송하는 큐를 의미합니다. 
// 이 모듈을 사용하면, 
// Rust 프로그래머는 mpsc 큐와 관련된 데이터 전송 작업을 효율적으로 수행할 수 있습니다.
use tokio::sync::mpsc;

// pub static KEYS: Lazy<identity::Keypair> = Lazy::new(identity::Keypair::generate_ed25519);
// pub static 피어_아이디: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
// pub static 체인_토픽: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
// pub static 블록_토픽: Lazy<Topic> = Lazy::new(|| Topic::new("블록들"));

pub static KEYS: Lazy<identity::Keypair> = Lazy::new(identity::Keypair::generate_ed25519);
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("블록들"));



// #[derive(Debug, Serialize, Deserialize)]는 Rust의 특수한 문법입니다. 
// 이 구문은 로칼_체인_요청_구조체 구조체에 Debug, Serialize, Deserialize 
// 특성을 상속하라는 것을 뜻합니다.
// Debug 특성은 구조체를 디버깅할 때 편리하게 사용할 수 있도록 하며, Serialize 특성은 
// 구조체를 직렬화할 수 있는 기능을 제공하고, Deserialize 특성은 직렬화된 데이터를 
// 역직렬화할 수 있는 기능을 제공한다.


#[derive(Debug, Serialize, Deserialize)]
pub struct 체인_반응_구조체 {
    pub 블록들: Vec<블록>,
    pub 수신자: String,
}



#[derive(Debug, Serialize, Deserialize)]
pub enum 로칼_체인_요청_구조체 {
    pub 출처_peer_id: String,
}

pub enum 이벤트_유형_열거형_데이타 {
    로컬_체인_반응(체인_반응_구조체),
    Input(String),
    Init,
}

#[derive(NetworkBehaviour)]
pub struct 앱동작_구조체 {
    pub floodsub: Floodsub,
    pub mdns: Mdns,
    #[behaviour(ignore)]//컴파일러에게 해당 코드를 무시하도록 알려주는 것입니다. "behaviour"은 특정 기능을 정의하는 Rust 플러그인이고, "ignore"는 그 플러그인에서 정의한 기능 중 하나입니다.
    pub 반응_송신자: mpsc::UnboundedSender<체인_반응_구조체>,
    #[behaviour(ignore)]
    pub 초기_송신자: mpsc::UnboundedSender<bool>,
    #[behaviour(ignore)]
    pub app: 앱,
}

impl 앱동작_구조체 {
    pub async fn new(
        app: 앱,
        반응_송신자: mpsc::UnboundedSender<체인_반응_구조체>,
        초기_송신자: mpsc::UnboundedSender<bool>,
    ) -> Self {
        let mut behaviour = Self {
            app,
            floodsub: Floodsub::new(*피어_아이디),
            mdns: Mdns::new(Default::default())
                .await
                .expect("mdns를 만들 수 없음"),
            반응_송신자,
            초기_송신자,
        };
        // behaviour.floodsub.subscribe의 의미는 Floodsub 모듈의 subscribe 기능을 호출하고, 
        // 해당 기능을 통해 특정 토픽에 대한 구독을 수행하는 것을 의미합니다.이 구독은 libp2p 
        // 네트워크에 있는 다른 피어들에서 전달되는 해당 토픽에 대한 메시지를 받게 됩니다. 
        // Floodsub 모듈은 libp2p 피어 간에 메시지를 퍼뜨리는 (flood) 프로토콜을 제공하며, 
        // 이 구독 기능은 메시지를 받을 수 있는 방식을 제공합니다.

        behaviour.floodsub.subscribe(체인_토픽.clone());
        behaviour.floodsub.subscribe(블록_토픽.clone());

        behaviour
    }
}

// 수신 이벤트 핸들러

// NetworkBehaviourEventProcess 라는 모듈은 네트워크 동작 이벤트를 처리하는 모듈로서
// libp2p::swarm::NetworkBehaviourEventProcess 를 통해 가져와 사용
// Floodsub 모듈에서 inject_event 함수의 기능은 Floodsub 플러그인의 상태에 이벤트를 
// 주입하는 것입니다. 
// 이 함수는 Floodsub 플러그인의 상태를 업데이트하고, 이벤트에 대한 처리를 수행합니다. 
// 예를 들어, 새로운 구독에 대한 알림, 메시지 수신 등의 작업을 포함할 수 있습니다.

impl NetworkBehaviourEventProcess<FloodsubEvent> for 앱동작_구조체 {
    fn inject_event(&mut self, event: FloodsubEvent) {
        if let FloodsubEvent::Message(메세지) = event {
            if let Ok(응답) = serde_json::from_slice::<체인_반응_구조체>(&메세지.data) {
                if 응답.수신자 == 피어_아이디.to_string() {
                    info!("{}에서의 응답:", 메세지.source);
                    응답.블록들.iter().for_each(|r| info!("{:?}", r));

                    self.app.블록들 = self.app.체인_선택_함수(self.app.블록들.clone(), 응답.블록들);
                }
            } else if let Ok(응답) = serde_json::from_slice::<로칼_체인_요청_구조체>(&메세지.data) {
                info!("로칼 체인을 {}에 보내는 중", 메세지.source.to_string());
                let peer_id = 응답.출처_peer_id;
                if 피어_아이디.to_string() == peer_id {
                    if let Err(e) = self.반응_송신자.send(체인_반응_구조체 {
                        블록들: self.app.블록들.clone(),
                        수신자: 메세지.source.to_string(),
                    }) {
                        error!("채널로 반응을 보내는데 에러발생, {}", e);
                    }
                }
            } else if let Ok(block) = serde_json::from_slice::<블록>(&메세지.data) {
                info!("{} 에서 새로운 블록을 받음", 메세지.source.to_string());
                self.app.블록_추가시도_함수(block);
            }
        }
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for 앱동작_구조체 {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(발견된_노드_목록) => {
                for (peer, _addr) in 발견된_노드_목록 {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(만료된_노드_목록) => {
                for (peer, _addr) in 만료된_노드_목록 {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

pub fn peer_목록_얻기(swarm: &Swarm<앱동작_구조체>) -> Vec<String> {
    info!("발견된 피어들:");
    let 노드들 = swarm.behaviour().mdns.discovered_nodes();//네트워크에서 찾은 노드 목록을 노드들 변수에 할당
    // let mut 중복되지않는_피어 = HashSet::new();은 러스트 코드에서 해쉬 세트 타입의 변수 중복되지않는_피어를 선언하고 생성하는 구문입니다. HashSet은 중복을 허용하지 않는 컬렉션입니다. 중복되지 않는 값들을 보관하고자 할 때 사용합니다. new 메소드는 빈 해쉬 세트를 생성하고 반환합니다.

    // 예를 들어, 중복되지않는_피어.insert(1), 중복되지않는_피어.insert(2)를 호출한 뒤, 중복되지않는_피어.insert(2)를 호출하면, 해쉬 세트에는 2개의 값 1과 2만이 남습니다. 만약 해쉬 세트가 아닌 다른 컬렉션에서 같은 값 2를 추가하면, 2개의 같은 값 2가 있게 됩니다.
    let mut 중복되지않는_피어 = HashSet::new();
    for peer in 노드들 {
        중복되지않는_피어.insert(peer);
    }
    중복되지않는_피어.iter().map(|p| p.to_string()).collect()
}

//아래의 rust 함수는 Swarm<앱동작_구조체> 타입의 참조자 swarm을 인자로 받아, peer 목록을 얻어와 각 peer를 출력하는 함수입니다.
pub fn 연결된_peer_출력_함수(swarm: &Swarm<앱동작_구조체>) {
    let peers = peer_목록_얻기(swarm);
    peers.iter().for_each(|p| info!("{}", p));
}

pub fn 체인_출력_처리_함수(swarm: &Swarm<앱동작_구조체>) {
    info!("로컬 블록체인:");
    let 블록_json =
    // "serde_json::to_string_pretty()" 는 Rust에서 serde_json 라이브러리에 정의된 메소드입니다. "to_string_pretty"은 serde_json 라이브러리를 사용하여 데이터를 JSON 문자열로 변환하는 메소드입니다. 이 메소드는 변환된 JSON 문자열을 보기 좋게 정렬합니다.
        serde_json::to_string_pretty(&swarm.behaviour().app.블록들).expect("블록들을 json으로 변환할 수 있음");
    info!("{}", 블록_json);
}

pub fn 새_블록_생성_처리_함수(터미날: &str, swarm: &mut Swarm<앱동작_구조체>) {
// "Some"은 Rust에서 제공하는 표준 라이브러리의 타입인 Option의 부분 타입입니다. 
// "Option"은 어떤 값이 존재할 수도 아닐 수도 있는 경우에 사용됩니다.
// Some(데이터)는 Option 타입으로서 값이 존재한다는 의미입니다.만약 값이 존재하지 않으면 None을 반환합니다.
// "strip_prefix" 메소드는 해당 문자열의 앞에서부터 지정된 문자열을 제거하는 메소드
    if let Some(데이터) = 터미날.strip_prefix("create b") {
        let behaviour = swarm.behaviour_mut();
        let 마지막_블록 = behaviour
            .app
            .블록들
            .last()
            .expect("적어도 하나의 블록이 있어야 합니다");
        let block = 블록::new(
            마지막_블록.id + 1,
            마지막_블록.해시.clone(),
            데이터.to_owned(),
        );
        // 아래 줄은 Rust에서 serde_json 라이브러리를 사용하여 블록 객체를 
        // json 형식으로 변환하는 과정.
        // 결과적으로 json 변수에는 블록 객체를 json 형식으로 변환한 결과가 저장
        let json = serde_json::to_string(&block).expect("블록들을 json으로 변환할 수 있음");
        behaviour.app.블록들.push(block);
        info!("새 블록을 broadcast 합니다");
        behaviour
            .floodsub
            .publish(블록_토픽.clone(), json.as_bytes());//json.as_bytes()는 json 문자열을 바이트 배열로 변환하는 메소드입니다. 이 메소드를 호출하면, json 문자열이 u8 타입의 배열로 변환됩니다. 예를 들어, "hello" 라는 문자열이 있다면 as_bytes()를 호출하면 [104, 101, 108, 108, 111]이 됩니다.
    }
}

========================shop







use super::{앱, 블록};
use libp2p::{
    NetworkBehaviour,
    identity,
    mdns::{Mdns, MdnsEvent},
    swarm::{
        NetworkBehaviourEventProcess, Swarm,
    },
    PeerId,
    floodsub::{Floodsub, FloodsubEvent, Topic},
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
pub struct 체인_반응_구조체 {
    pub 블록들: Vec<블록>,
    pub 수신자: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct 로칼_체인_요청_구조체 {
    pub 출처_peer_id: String,
}

pub enum 이벤트_유형_열거형_데이타 {
    로컬_체인_반응(체인_반응_구조체),
    Input(String),
    Init,
}

#[derive(NetworkBehaviour)]
pub struct 앱동작_구조체 {
    pub floodsub: Floodsub,
    pub mdns: Mdns,
    #[behaviour(ignore)]//컴파일러에게 해당 코드를 무시하도록 알려주는 것입니다. "behaviour"은 특정 기능을 정의하는 Rust 플러그인이고, "ignore"는 그 플러그인에서 정의한 기능 중 하나입니다.
    pub 반응_송신자: mpsc::UnboundedSender<체인_반응_구조체>,
    #[behaviour(ignore)]
    pub 초기_송신자: mpsc::UnboundedSender<bool>,
    #[behaviour(ignore)]
    pub app: 앱,
}

impl 앱동작_구조체 {
    pub async fn new(
        app: 앱,
        반응_송신자: mpsc::UnboundedSender<체인_반응_구조체>,
        초기_송신자: mpsc::UnboundedSender<bool>,
    ) -> Self {
        let mut behaviour = Self {
            app,
            floodsub: Floodsub::new(*PEER_ID),
            mdns: Mdns::new(Default::default())
                .await
                .expect("mdns를 만들 수 없음"),
            반응_송신자,
            초기_송신자,
        };
        // behaviour.floodsub.subscribe의 의미는 Floodsub 모듈의 subscribe 기능을 호출하고, 
        // 해당 기능을 통해 특정 토픽에 대한 구독을 수행하는 것을 의미합니다.이 구독은 libp2p 
        // 네트워크에 있는 다른 피어들에서 전달되는 해당 토픽에 대한 메시지를 받게 됩니다. 
        // Floodsub 모듈은 libp2p 피어 간에 메시지를 퍼뜨리는 (flood) 프로토콜을 제공하며, 
        // 이 구독 기능은 메시지를 받을 수 있는 방식을 제공합니다.

        behaviour.floodsub.subscribe(CHAIN_TOPIC.clone());
        behaviour.floodsub.subscribe(BLOCK_TOPIC.clone());

        behaviour
    }
}

// 수신 이벤트 핸들러

// NetworkBehaviourEventProcess 라는 모듈은 네트워크 동작 이벤트를 처리하는 모듈로서
// libp2p::swarm::NetworkBehaviourEventProcess 를 통해 가져와 사용
// Floodsub 모듈에서 inject_event 함수의 기능은 Floodsub 플러그인의 상태에 이벤트를 
// 주입하는 것입니다. 
// 이 함수는 Floodsub 플러그인의 상태를 업데이트하고, 이벤트에 대한 처리를 수행합니다. 
// 예를 들어, 새로운 구독에 대한 알림, 메시지 수신 등의 작업을 포함할 수 있습니다.

impl NetworkBehaviourEventProcess<FloodsubEvent> for 앱동작_구조체 {
    fn inject_event(&mut self, event: FloodsubEvent) {
        // FloodsubEvent::Message인 경우에는 if, else if로 처리하면서, 
        //나머지 경우에는 _로 처리하도록 했습니다. _는 모든 패턴에 매치되는 와일드카드 패턴입니다.
        match event {
            FloodsubEvent::Message(message) => {
                if let Ok(response) = serde_json::from_slice::<체인_반응_구조체>(&message.data) {
                    if response.수신자 == PEER_ID.to_string() {
                        info!("{}에서의 응답:", message.source);
                        response.블록들.iter().for_each(|r| info!("{:?}", r));
        
                        self.app.블록들 = self.app.체인_선택_함수(self.app.블록들.clone(), response.블록들);
                    }
                } else if let Ok(response) = serde_json::from_slice::<로칼_체인_요청_구조체>(&message.data) {
                    info!("로칼 체인을 {}에 보내는 중", message.source.to_string());
                    let peer_id = response.출처_peer_id;
                    if PEER_ID.to_string() == peer_id {
                        if let Err(e) = self.반응_송신자.send(체인_반응_구조체 {
                            블록들: self.app.블록들.clone(),
                            수신자: message.source.to_string(),
                        }) {
                            error!("채널로 반응을 보내는데 에러발생, {}", e);
                        }
                    }
                } else if let Ok(block) = serde_json::from_slice::<블록>(&message.data) {
                    info!("{} 에서 새로운 블록을 받음", message.source.to_string());
                    self.app.블록_추가시도_함수(block);
                }
            }
            _ => {}
        }
        
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for 앱동작_구조체 {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(발견된_노드_목록) => {
                for (peer, _addr) in 발견된_노드_목록 {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(만료된_노드_목록) => {
                for (peer, _addr) in 만료된_노드_목록 {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

pub fn peer_목록_얻기(swarm: &Swarm<앱동작_구조체>) -> Vec<String> {
    info!("발견된 피어들:");
    let nodes = swarm.behaviour().mdns.discovered_nodes();//네트워크에서 찾은 노드 목록을 nodes 변수에 할당
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }
    unique_peers.iter().map(|p| p.to_string()).collect()
}

//아래의 rust 함수는 Swarm<앱동작_구조체> 타입의 참조자 swarm을 인자로 받아, peer 목록을 얻어와 각 peer를 출력하는 함수입니다.
pub fn 연결된_peer_출력_함수(swarm: &Swarm<앱동작_구조체>) {
    let peers = peer_목록_얻기(swarm);
    peers.iter().for_each(|p| info!("{}", p));
}

pub fn 체인_출력_처리_함수(swarm: &Swarm<앱동작_구조체>) {
    info!("로컬 블록체인:");
    let 블록_json =
        serde_json::to_string_pretty(&swarm.behaviour().app.블록들).expect("블록들을 json으로 변환할 수 있음");
    info!("{}", 블록_json);
}
//
pub fn 새_블록_생성_처리_함수(cmd: &str, swarm: &mut Swarm<앱동작_구조체>) {
    match cmd.strip_prefix("new block") {
        Some(데이터) => {
            let behaviour = swarm.behaviour_mut();
            let 마지막_블록 = behaviour
                .app
                .블록들
                .last()
                .expect("적어도 하나의 블록이 있어야 합니다");
            let block = 블록::new(
                마지막_블록.id + 1,
                마지막_블록.해시.clone(),
                데이터.to_owned(),
            );

            let json = serde_json::to_string(&block).expect("블록들을 json으로 변환할 수 있음");
            behaviour.app.블록들.push(block);
            info!("새 블록을 broadcast 합니다");
            behaviour
                .floodsub
                .publish(BLOCK_TOPIC.clone(), json.as_bytes());
        },
        None => {},
    }
}




