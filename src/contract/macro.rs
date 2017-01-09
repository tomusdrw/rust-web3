#[cfg(test)]
mod tests {
  use api;
  use helpers::tests::TestTransport;
  use types::U256;

  contract! {
    contract Token {
        event Transfer(address indexed from, address indexed to, uint256 value);
        event Approval(address indexed owner, address indexed spender, uint256 value);

        function balanceOf(address _owner) constant returns (uint256 balance);
        function transfer(address _to, uint256 _value) returns (bool success);
        function transferFrom(address _from, address _to, uint256 _value) returns (bool success);
        function approve(address _spender, uint256 _value) returns (bool success);
        function allowance(address _owner, address _spender) constant returns (uint256 remaining);
    }
  }

  #[test]
  fn should_call_token_contract {
    // given
    let transport = helpers::tests::TestTransport::default();
    let eth = api::Eth::new(transport);
    let token = Token::at(1.into(), eth);

    // when
    let balance = token.balanceOf(5.into());

    // then
    assert_eq!(balance, U256::from(10));

  }
}

