use std::collections::HashMap;
use crate::types::{Transaction};
use serde::{Deserialize, Serialize};
use crate::types::U64;

/// Transaction Pool Content Info
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct TxpoolContentInfo {
    /// pending tx
    pub pending: HashMap<String, HashMap<String, Transaction>>,
    /// queued tx
    pub queued: HashMap<String, HashMap<String, Transaction>>,
}

/// Transaction Pool Inspect Info
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct TxpoolInspectInfo {
    /// pending tx
    pub pending: HashMap<String, HashMap<String, String>>,
    /// queued tx
    pub queued: HashMap<String, HashMap<String, String>>,
}

/// Transaction Pool Status
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct TxpoolStatus {
    /// number of pending tx
    pub pending: U64,
    /// number of queued tx
    pub queued: U64,
}

#[cfg(test)]
mod tests {
    use super::{TxpoolStatus, TxpoolContentInfo, TxpoolInspectInfo};
    use serde_json;

    #[test]
    fn should_deserialize_txpool_content() {
        let txpool_content_str = r#"{
            "pending": {
                "0x0513Dc7403e074F5C77368eE2819fA3A65b5cf80": {
                    "6712": {
                        "blockHash": null,
                        "blockNumber": null,
                        "from": "0x0513dc7403e074f5c77368ee2819fa3a65b5cf80",
                        "gas": "0x186a0",
                        "gasPrice": "0x2cb417800",
                        "hash": "0xc463c2dcab885136f76d093357f62b0541d1bfa4e96f27f413a7191cc625e105",
                        "input": "0x",
                        "nonce": "0x1a38",
                        "to": "0x0b9ab0cce5238c24ea25ee3d921865da818ccf5e",
                        "transactionIndex": null,
                        "value": "0x1",
                        "v": "0x29",
                        "r": "0x7199ee383eb3c2869069ecaabcea8dae4257e8f37a7c3e23e5d781c676d2c7b7",
                        "s": "0x6afc7e6753e90dff25a002183364f6cd564c8ccd0a1b0381214c4bb4e59a843b"
                    }
                },
                "0x07e80128C7a35D0d43DDCC67fa8B1495871e08bf": {
                    "41588": {
                        "blockHash": null,
                        "blockNumber": null,
                        "from": "0x07e80128c7a35d0d43ddcc67fa8b1495871e08bf",
                        "gas": "0xc074c",
                        "gasPrice": "0xee6b2800",
                        "hash": "0x73057ec83d040f5d3be8afae35b447d7996472b5dedf2e727c8f4a2e1bedca14",
                        "input": "0x608060405234801561001057600080fd5b50604051610d54380380610d548339818101604052602081101561003357600080fd5b8101908080519060200190929190505050336000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555080600260006101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555050610c7f806100d56000396000f3fe6080604052600436106100705760003560e01c80638feb1b8b1161004e5780638feb1b8b146101fd578063d4ee1d901461024e578063e45bf7a6146102a5578063f2fde38b146102fc57610070565b806359a006801461013e57806379ba50971461018f5780638da5cb5b146101a6575b600260009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff166108fc349081150290604051600060405180830381858888f193505050501580156100d8573d6000803e3d6000fd5b507f0fe4cb1d003e6b2859d9f82ed185534d04565d376652186cbd07c0105fdcc5d830604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390a1005b34801561014a57600080fd5b5061018d6004803603602081101561016157600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919050505061034d565b005b34801561019b57600080fd5b506101a4610664565b005b3480156101b257600080fd5b506101bb610801565b604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390f35b34801561020957600080fd5b5061024c6004803603602081101561022057600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610826565b005b34801561025a57600080fd5b50610263610b61565b604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390f35b3480156102b157600080fd5b506102ba610b87565b604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390f35b34801561030857600080fd5b5061034b6004803603602081101561031f57600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610bad565b005b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16146103a657600080fd5b600081905060008173ffffffffffffffffffffffffffffffffffffffff166370a08231306040518263ffffffff1660e01b8152600401808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060206040518083038186803b15801561042a57600080fd5b505afa15801561043e573d6000803e3d6000fd5b505050506040513d602081101561045457600080fd5b81019080805190602001909291905050509050600081116104dd576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040180806020018281038252601e8152602001807f62616c616e6365206d7573742062652067726561746572207468616e2030000081525060200191505060405180910390fd5b8173ffffffffffffffffffffffffffffffffffffffff1663a9059cbb600260009054906101000a900473ffffffffffffffffffffffffffffffffffffffff16836040518363ffffffff1660e01b8152600401808373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200182815260200192505050600060405180830381600087803b15801561058657600080fd5b505af115801561059a573d6000803e3d6000fd5b505050507f8664be48506bd501d568d732361f45a27336ed6ea23c69c994d33e971ff7f40130600260009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1683604051808473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020018373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001828152602001935050505060405180910390a1505050565b600160009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16146106be57600080fd5b600160009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff166000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff167f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e060405160405180910390a3600160009054906101000a900473ffffffffffffffffffffffffffffffffffffffff166000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055506000600160006101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff160217905550565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff161461087f57600080fd5b600081905060008173ffffffffffffffffffffffffffffffffffffffff166370a08231306040518263ffffffff1660e01b8152600401808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060206040518083038186803b15801561090357600080fd5b505afa158015610917573d6000803e3d6000fd5b505050506040513d602081101561092d57600080fd5b81019080805190602001909291905050509050600081116109b6576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040180806020018281038252601e8152602001807f62616c616e6365206d7573742062652067726561746572207468616e2030000081525060200191505060405180910390fd5b8173ffffffffffffffffffffffffffffffffffffffff1663a9059cbb600260009054906101000a900473ffffffffffffffffffffffffffffffffffffffff16836040518363ffffffff1660e01b8152600401808373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200182815260200192505050602060405180830381600087803b158015610a5f57600080fd5b505af1158015610a73573d6000803e3d6000fd5b505050506040513d6020811015610a8957600080fd5b8101908080519060200190929190505050507f8664be48506bd501d568d732361f45a27336ed6ea23c69c994d33e971ff7f40130600260009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1683604051808473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020018373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001828152602001935050505060405180910390a1505050565b600160009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b600260009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff1614610c0657600080fd5b80600160006101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055505056fea265627a7a7231582001543b5939e998cc829c177eb8dd2927268ba9f47e41ce006f6276379d324b6f64736f6c634300050c0032000000000000000000000000e8bb7d0000e0b8f7114863d7fee666b5270111b8",
                        "nonce": "0xa274",
                        "to": null,
                        "transactionIndex": null,
                        "value": "0x0",
                        "v": "0x1c",
                        "r": "0x36cdfb4894e117f27ff7153b6acbd1054464806653326e66706cc8943808e898",
                        "s": "0x4e7f6439db6a3f6c645e063c0aabaf2d7d5a1c6bba0982b8cf4d45b798593eff"
                    },
                    "41589": {
                        "blockHash": null,
                        "blockNumber": null,
                        "from": "0x07e80128c7a35d0d43ddcc67fa8b1495871e08bf",
                        "gas": "0xc074c",
                        "gasPrice": "0xee6b2800",
                        "hash": "0xc67949dfcf2e5cbb054f0711d5dbf1789801303773c85b7d0b3a8108832b99b0",
                        "input": "0x608060405234801561001057600080fd5b50604051610d54380380610d548339818101604052602081101561003357600080fd5b8101908080519060200190929190505050336000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555080600260006101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555050610c7f806100d56000396000f3fe6080604052600436106100705760003560e01c80638feb1b8b1161004e5780638feb1b8b146101fd578063d4ee1d901461024e578063e45bf7a6146102a5578063f2fde38b146102fc57610070565b806359a006801461013e57806379ba50971461018f5780638da5cb5b146101a6575b600260009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff166108fc349081150290604051600060405180830381858888f193505050501580156100d8573d6000803e3d6000fd5b507f0fe4cb1d003e6b2859d9f82ed185534d04565d376652186cbd07c0105fdcc5d830604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390a1005b34801561014a57600080fd5b5061018d6004803603602081101561016157600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919050505061034d565b005b34801561019b57600080fd5b506101a4610664565b005b3480156101b257600080fd5b506101bb610801565b604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390f35b34801561020957600080fd5b5061024c6004803603602081101561022057600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610826565b005b34801561025a57600080fd5b50610263610b61565b604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390f35b3480156102b157600080fd5b506102ba610b87565b604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390f35b34801561030857600080fd5b5061034b6004803603602081101561031f57600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610bad565b005b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16146103a657600080fd5b600081905060008173ffffffffffffffffffffffffffffffffffffffff166370a08231306040518263ffffffff1660e01b8152600401808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060206040518083038186803b15801561042a57600080fd5b505afa15801561043e573d6000803e3d6000fd5b505050506040513d602081101561045457600080fd5b81019080805190602001909291905050509050600081116104dd576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040180806020018281038252601e8152602001807f62616c616e6365206d7573742062652067726561746572207468616e2030000081525060200191505060405180910390fd5b8173ffffffffffffffffffffffffffffffffffffffff1663a9059cbb600260009054906101000a900473ffffffffffffffffffffffffffffffffffffffff16836040518363ffffffff1660e01b8152600401808373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200182815260200192505050600060405180830381600087803b15801561058657600080fd5b505af115801561059a573d6000803e3d6000fd5b505050507f8664be48506bd501d568d732361f45a27336ed6ea23c69c994d33e971ff7f40130600260009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1683604051808473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020018373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001828152602001935050505060405180910390a1505050565b600160009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16146106be57600080fd5b600160009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff166000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff167f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e060405160405180910390a3600160009054906101000a900473ffffffffffffffffffffffffffffffffffffffff166000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055506000600160006101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff160217905550565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff161461087f57600080fd5b600081905060008173ffffffffffffffffffffffffffffffffffffffff166370a08231306040518263ffffffff1660e01b8152600401808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060206040518083038186803b15801561090357600080fd5b505afa158015610917573d6000803e3d6000fd5b505050506040513d602081101561092d57600080fd5b81019080805190602001909291905050509050600081116109b6576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040180806020018281038252601e8152602001807f62616c616e6365206d7573742062652067726561746572207468616e2030000081525060200191505060405180910390fd5b8173ffffffffffffffffffffffffffffffffffffffff1663a9059cbb600260009054906101000a900473ffffffffffffffffffffffffffffffffffffffff16836040518363ffffffff1660e01b8152600401808373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200182815260200192505050602060405180830381600087803b158015610a5f57600080fd5b505af1158015610a73573d6000803e3d6000fd5b505050506040513d6020811015610a8957600080fd5b8101908080519060200190929190505050507f8664be48506bd501d568d732361f45a27336ed6ea23c69c994d33e971ff7f40130600260009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1683604051808473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020018373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001828152602001935050505060405180910390a1505050565b600160009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b600260009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff1614610c0657600080fd5b80600160006101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055505056fea265627a7a7231582001543b5939e998cc829c177eb8dd2927268ba9f47e41ce006f6276379d324b6f64736f6c634300050c0032000000000000000000000000e8bb7d0000e0b8f7114863d7fee666b5270111b8",
                        "nonce": "0xa275",
                        "to": null,
                        "transactionIndex": null,
                        "value": "0x0",
                        "v": "0x1b",
                        "r": "0x6b4a870237b8fb2acf4b5c5c52fc0bf0ac4f162acc17f78e113f8a9839ca7103",
                        "s": "0x5d8d56e792ef6f6bf664978ba8a3e8f4fdb2ef040d9c191397734e52339b917"
                    },
                    "41590": {
                        "blockHash": null,
                        "blockNumber": null,
                        "from": "0x07e80128c7a35d0d43ddcc67fa8b1495871e08bf",
                        "gas": "0xc074c",
                        "gasPrice": "0xee6b2800",
                        "hash": "0x87f1eca993dd77d4fcf34aaa078f555dde68d478c7fcc75afefbc06553bde807",
                        "input": "0x608060405234801561001057600080fd5b50604051610d54380380610d548339818101604052602081101561003357600080fd5b8101908080519060200190929190505050336000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555080600260006101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555050610c7f806100d56000396000f3fe6080604052600436106100705760003560e01c80638feb1b8b1161004e5780638feb1b8b146101fd578063d4ee1d901461024e578063e45bf7a6146102a5578063f2fde38b146102fc57610070565b806359a006801461013e57806379ba50971461018f5780638da5cb5b146101a6575b600260009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff166108fc349081150290604051600060405180830381858888f193505050501580156100d8573d6000803e3d6000fd5b507f0fe4cb1d003e6b2859d9f82ed185534d04565d376652186cbd07c0105fdcc5d830604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390a1005b34801561014a57600080fd5b5061018d6004803603602081101561016157600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff16906020019092919050505061034d565b005b34801561019b57600080fd5b506101a4610664565b005b3480156101b257600080fd5b506101bb610801565b604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390f35b34801561020957600080fd5b5061024c6004803603602081101561022057600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610826565b005b34801561025a57600080fd5b50610263610b61565b604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390f35b3480156102b157600080fd5b506102ba610b87565b604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390f35b34801561030857600080fd5b5061034b6004803603602081101561031f57600080fd5b81019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610bad565b005b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16146103a657600080fd5b600081905060008173ffffffffffffffffffffffffffffffffffffffff166370a08231306040518263ffffffff1660e01b8152600401808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060206040518083038186803b15801561042a57600080fd5b505afa15801561043e573d6000803e3d6000fd5b505050506040513d602081101561045457600080fd5b81019080805190602001909291905050509050600081116104dd576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040180806020018281038252601e8152602001807f62616c616e6365206d7573742062652067726561746572207468616e2030000081525060200191505060405180910390fd5b8173ffffffffffffffffffffffffffffffffffffffff1663a9059cbb600260009054906101000a900473ffffffffffffffffffffffffffffffffffffffff16836040518363ffffffff1660e01b8152600401808373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200182815260200192505050600060405180830381600087803b15801561058657600080fd5b505af115801561059a573d6000803e3d6000fd5b505050507f8664be48506bd501d568d732361f45a27336ed6ea23c69c994d33e971ff7f40130600260009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1683604051808473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020018373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001828152602001935050505060405180910390a1505050565b600160009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16146106be57600080fd5b600160009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff166000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff167f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e060405160405180910390a3600160009054906101000a900473ffffffffffffffffffffffffffffffffffffffff166000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055506000600160006101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff160217905550565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff161461087f57600080fd5b600081905060008173ffffffffffffffffffffffffffffffffffffffff166370a08231306040518263ffffffff1660e01b8152600401808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060206040518083038186803b15801561090357600080fd5b505afa158015610917573d6000803e3d6000fd5b505050506040513d602081101561092d57600080fd5b81019080805190602001909291905050509050600081116109b6576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040180806020018281038252601e8152602001807f62616c616e6365206d7573742062652067726561746572207468616e2030000081525060200191505060405180910390fd5b8173ffffffffffffffffffffffffffffffffffffffff1663a9059cbb600260009054906101000a900473ffffffffffffffffffffffffffffffffffffffff16836040518363ffffffff1660e01b8152600401808373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200182815260200192505050602060405180830381600087803b158015610a5f57600080fd5b505af1158015610a73573d6000803e3d6000fd5b505050506040513d6020811015610a8957600080fd5b8101908080519060200190929190505050507f8664be48506bd501d568d732361f45a27336ed6ea23c69c994d33e971ff7f40130600260009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1683604051808473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020018373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001828152602001935050505060405180910390a1505050565b600160009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b600260009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff1614610c0657600080fd5b80600160006101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055505056fea265627a7a7231582001543b5939e998cc829c177eb8dd2927268ba9f47e41ce006f6276379d324b6f64736f6c634300050c0032000000000000000000000000e8bb7d0000e0b8f7114863d7fee666b5270111b8",
                        "nonce": "0xa276",
                        "to": null,
                        "transactionIndex": null,
                        "value": "0x0",
                        "v": "0x1b",
                        "r": "0x1ac3d79b07adb8c214d9531d155cab4d9f983128e2e6237108d344df85ebbf",
                        "s": "0xe2b072c52085aea3cc1cf9f0284ef6ffc42d2daebb156b4c995876bcd474b85"
                    }
                }
            },
            "queued": {
                "0x0f87ffCD71859233Eb259f42b236C8e9873444e3": {
                    "7": {
                        "blockHash": null,
                        "blockNumber": null,
                        "from": "0x0f87ffcd71859233eb259f42b236c8e9873444e3",
                        "gas": "0x5208",
                        "gasPrice": "0x2540be400",
                        "hash": "0x5c2cc0e17ea6c48489fddd2a64975791e0d4a7cc0ae4a81613682fd134be1baa",
                        "input": "0x",
                        "nonce": "0x7",
                        "to": "0x3479be69e07e838d9738a301bb0c89e8ea2bef4a",
                        "transactionIndex": null,
                        "value": "0x38d7ea4c68000",
                        "v": "0x1c",
                        "r": "0x2e6531dc90a8bd2edeeb6aa8ec0d86e5ab6ebfe00fda91a5b8aea7249f0ea040",
                        "s": "0x1aebe1e2f1b0bd18f2936e09fc54c49c42e9e3344444e1ffefabce472eb27dfe"
                    },
                    "8": {
                        "blockHash": null,
                        "blockNumber": null,
                        "from": "0x0f87ffcd71859233eb259f42b236c8e9873444e3",
                        "gas": "0x5208",
                        "gasPrice": "0x2540be400",
                        "hash": "0x8755fadda87e9fd2e66c0bfa542baa9f552cddda334f673e272f3aa686efb5e4",
                        "input": "0x",
                        "nonce": "0x8",
                        "to": "0x73aaf691bc33fe38f86260338ef88f9897ecaa4f",
                        "transactionIndex": null,
                        "value": "0x38d7ea4c68000",
                        "v": "0x1b",
                        "r": "0x8a1c9248321337c460f8286a0b15ce302f03f9e9b702865c1762f0f03018857",
                        "s": "0x4655ee7fb06bf169e9fff633916bb011565520aa5a3372d4cb3805001c7a18af"
                    }
                },
                "0x307e8f249bCCcfa5b245449256C5d7e6e079943e": {
                    "3": {
                        "blockHash": null,
                        "blockNumber": null,
                        "from": "0x307e8f249bcccfa5b245449256c5d7e6e079943e",
                        "gas": "0x5208",
                        "gasPrice": "0x2540be400",
                        "hash": "0x54ea4d4905bf74b687ccc73e8a1fb9615357e5e82d3f716e7ab10cd8460a3221",
                        "input": "0x",
                        "nonce": "0x3",
                        "to": "0x73aaf691bc33fe38f86260338ef88f9897ecaa4f",
                        "transactionIndex": null,
                        "value": "0x2386f26fc10000",
                        "v": "0x1b",
                        "r": "0xc9441e267c12a03ec2e833e21737d4bca080bc6ce13790113a207dcd06f76b5e",
                        "s": "0x63801a9faacff49bba85c45a5b8a0e0a4f37b15240bfc19c5000be885d056e27"
                    }
                }
            }
        }"#;
        let _: TxpoolContentInfo = serde_json::from_str(txpool_content_str).unwrap();
    }

    #[test]
    fn should_deserialize_txpool_inspect() {
        let txpool_inspect_str = r#"{
            "pending": {
                "0x0512261A7486b1e29704ac49A5EB355b6Fd86872": {
                    "124930": "0x000000000000000000000000000000000000007E: 0 wei + 100187 gas × 20000000000 wei"
                 },
                "0x201354729f8d0f8b64e9a0c353c672C6a66B3857": {
                    "252350": "0xd10e3Be2bc8f959Bc8C41CF65F60dE721cF89ADF: 0 wei + 65792 gas × 2000000000 wei",
                    "252351": "0xd10e3Be2bc8f959Bc8C41CF65F60dE721cF89ADF: 0 wei + 65792 gas × 2000000000 wei",
                    "252352": "0xd10e3Be2bc8f959Bc8C41CF65F60dE721cF89ADF: 0 wei + 65780 gas × 2000000000 wei",
                    "252353": "0xd10e3Be2bc8f959Bc8C41CF65F60dE721cF89ADF: 0 wei + 65780 gas × 2000000000 wei"
                }
            },
            "queued": {
                "0x0f87ffCD71859233Eb259f42b236C8e9873444e3": {
                    "7": "0x3479BE69e07E838D9738a301Bb0c89e8EA2Bef4a: 1000000000000000 wei + 21000 gas × 10000000000 wei",
                    "8": "0x73Aaf691bc33fe38f86260338EF88f9897eCaa4F: 1000000000000000 wei + 21000 gas × 10000000000 wei"
                },
                "0x307e8f249bCCcfa5b245449256C5d7e6e079943e": {
                    "3": "0x73Aaf691bc33fe38f86260338EF88f9897eCaa4F: 10000000000000000 wei + 21000 gas × 10000000000 wei"
                }
            }
        }"#;
        let _: TxpoolInspectInfo = serde_json::from_str(txpool_inspect_str).unwrap();
    }

    #[test]
    fn should_deserialize_txpool_status() {
        let txpool_status_str = r#"{
            "pending":"0x23",
            "queued":"0x20"
        }"#;
        let _: TxpoolStatus = serde_json::from_str(txpool_status_str).unwrap();
    }
}
