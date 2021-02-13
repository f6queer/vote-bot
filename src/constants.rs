pub const CONFIG_PATH: &'static str = "Bot.toml";
pub const DB_PATH: &'static str = "bot.db";
pub const TITLE_NAME: &'static str = "**F⁶ 임원 선거**";
pub const HELP: &'static str = "**F⁶ 투표봇 도움말**
`/start`: 봇을 시작합니다.
`/help`: 도움말을 표시합니다.
`/poll`: 현재 진행중인 투표를 보여줍니다.
`/vote [후보 번호]`: [후보 번호]에 해당하는 후보에 투표합니다.
`/undo [토큰]`: [토큰]을 통해서 투표를 취소합니다.
`/admin_help`: 관리자 전용 도움말을 표시합니다.
`/accept [토큰]`: 자신을 투표 가능한 유저로 등록합니다. 
`/accept_admin [토큰]`: 유효한 토큰을 통해서 자신을 관리자로 등록합니다.";
pub const ADMIN_HELP: &'static str = "**F⁶ 투표봇 관리자 도움말**
`/create_poll [후보 이름...] [진행할 시간(분)]`: 새로운 투표를 시작합니다.
`/remote_poll`: 현재 진행중인 투표를 종료합니다.
`/add_user`: 투표 가능한 유저를 추가하기 위한 토큰을 생성합니다.
`/list_user`: 투표 가능한 유저 목록을 보여줍니다.
`/remove_user [투표 가능한 유저 번호]`: 투표 가능한 유저를 삭제합니다.
`/add_admin`: 관리자를 등록하기 위한 토큰을 생성합니다.
`/list_admin`: 관리자 목록을 보여줍니다.
`/remove_admin [관리자 번호]`: 관리자를 삭제합니다.";
pub const AES_KEY_LEN: usize = 32;
pub const NONCE_LEN: usize = 12;
