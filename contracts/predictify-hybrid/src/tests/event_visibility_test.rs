#[test]
#[should_panic(expected = "User not allowlisted")]
fn non_allowlisted_user_cannot_bet_on_private_event() {
    let env = setup_env();

    let event = create_private_event(&env, vec![allowed_user.clone()]);
    
    place_bet(&env, non_allowed_user.clone(), event.id);
}

#[test]
fn allowlisted_user_can_bet_on_private_event() {
    let env = setup_env();

    let event = create_private_event(&env, vec![allowed_user.clone()]);
    
    let result = place_bet(&env, allowed_user.clone(), event.id);

    assert!(result.is_ok());
}

#[test]
fn public_event_allows_any_user() {
    let env = setup_env();

    let event = create_public_event(&env);

    let result = place_bet(&env, random_user.clone(), event.id);

    assert!(result.is_ok());
}

#[test]
#[should_panic]
fn private_event_with_empty_allowlist_blocks_everyone() {
    let env = setup_env();

    let event = create_private_event(&env, vec![]);
    
    place_bet(&env, user.clone(), event.id);
}