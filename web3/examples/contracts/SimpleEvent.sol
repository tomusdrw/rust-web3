pragma solidity ^0.4.0;

contract SimpleEvent {
    event Hello(address sender);

    function hello() public {
        emit Hello(msg.sender);
    }
}
