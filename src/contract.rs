use cosmwasm_std::{
    debug_print, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier,
    StdError, StdResult, Storage,
};

use crate::msg::{ReencryptionKeyResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{config, config_read, State};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    let state = State {
        reencryption_key: [0; 32],
        owner: deps.api.canonical_address(&env.message.sender)?,
    };

    config(&mut deps.storage).save(&state)?;

    debug_print!("Contract was initialized by {}", env.message.sender);

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Set { reencryption_key} => try_set_reencryption_key(deps, env, reencryption_key),
        HandleMsg::Reset { } => try_reset(deps, env),
    }
}

pub fn try_set_reencryption_key<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    key: [u8; 32]
) -> StdResult<HandleResponse> {
    let sender_address_raw = deps.api.canonical_address(&env.message.sender)?;

    config(&mut deps.storage).update(|mut state| {
        state.reencryption_key = key;
        Ok(state)
    })?;

    debug_print("reencryption key registered by {}");
    Ok(HandleResponse::default())
}

pub fn try_reset<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let sender_address_raw = deps.api.canonical_address(&env.message.sender)?;
    config(&mut deps.storage).update(|mut state| {
        if sender_address_raw != state.owner {
            return Err(StdError::Unauthorized { backtrace: None });
        }
        state.reencryption_key = [0;32];
        Ok(state)
    })?;
    debug_print("count reset successfully");
    Ok(HandleResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetReencryptionKey {} => to_binary(&query_count(deps)?),
    }
}

fn query_count<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<ReencryptionKeyResponse> {
    let state = config_read(&deps.storage).load()?;
    Ok(ReencryptionKeyResponse { reencryption_key: state.reencryption_key })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, StdError};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg { };
        let env = mock_env("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(&deps, QueryMsg::GetReencryptionKey {}).unwrap();
        let value: ReencryptionKeyResponse = from_binary(&res).unwrap();
        assert_eq!([0;32], value.reencryption_key);
    }

    #[test]
    fn set() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg { };
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        // anyone can set
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::Set {reencryption_key: [1;32]};
        let _res = handle(&mut deps, env, msg).unwrap();

        // should be set
        let res = query(&deps, QueryMsg::GetReencryptionKey {}).unwrap();
        let value: ReencryptionKeyResponse = from_binary(&res).unwrap();
        assert_eq!([1;32], value.reencryption_key);
    }

    #[test]
    fn reset() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        // not anyone can reset
        let unauth_env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::Reset {};
        let res = handle(&mut deps, unauth_env, msg);
        match res {
            Err(StdError::Unauthorized { .. }) => {}
            _ => panic!("Must return unauthorized error"),
        }

        // only the original creator can reset the counter
        let auth_env = mock_env("creator", &coins(2, "token"));
        let set_msg = HandleMsg::Set {reencryption_key: [55;32]};
        let set_res = handle(&mut deps, auth_env, set_msg).unwrap();

        // should now be 55
        let res = query(&deps, QueryMsg::GetReencryptionKey {}).unwrap();
        let value: ReencryptionKeyResponse = from_binary(&res).unwrap();
        assert_eq!([55;32], value.reencryption_key);

        // reset it now
        let auth_env = mock_env("creator", &coins(2, "token"));
        let reset_msg = HandleMsg::Reset {};
        let reset_res = handle(&mut deps, auth_env, reset_msg).unwrap();

        // should now be 0
        let res = query(&deps, QueryMsg::GetReencryptionKey {}).unwrap();
        let value: ReencryptionKeyResponse = from_binary(&res).unwrap();
        assert_eq!([0;32], value.reencryption_key);
    }
}
