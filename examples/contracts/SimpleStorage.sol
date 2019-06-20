pragma solidity ^0.5.9;

contract SimpleStorage {
    uint storedData;

    function set(uint x) public {
        require(false, "supposed to fail");
        storedData = x;
    }

    function get() public view returns (uint) {
        require(false, "supposed to fail");
        return storedData;
    }
}