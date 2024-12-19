// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;
pragma experimental ABIEncoderV2;

import "../lib/proxy/ERC1967/ERC1967Proxy.sol";

contract AtomicProxyDeploymentFactory {
    event ContractDeployed(address indexed contract_address);
    event FunctionCalled(
        address indexed target,
        bytes indexed data,
        bytes indexed result
    );


    /**
     * @dev Deploys an implementation contract and a proxy contract, then calls a set of functions through the proxy contract.
     * @param impl_init_code The init code for deploying the implementation contract.
     * @param impl_salt The salt for deploying the implementation contract using CREATE2.
     * @param proxy_salt The salt for deploying the proxy contract using CREATE2.
     * @param calls An array of function calls (encoded as {signature + arguments}) to execute on the proxy contract.
     */
    function deploy(
        bytes memory impl_init_code,
        bytes32 impl_salt,
        bytes32 proxy_salt,
        bytes[] memory calls
    ) external {
        // deploy impl
        address impl_address = _deployContract(
            impl_init_code,
            impl_salt
        );
        emit ContractDeployed(impl_address);

        // deploy proxy
        bytes memory proxy_init_code = abi.encodePacked(
            type(ERC1967Proxy).creationCode,
            abi.encode(impl_address, "")
        );
        address proxy_address = _deployContract(
            proxy_init_code,
            proxy_salt
        );
        emit ContractDeployed(proxy_address);

        // call initializers
        for (uint256 i = 0; i < calls.length; i++) {
            (bool success, bytes memory result) = proxy_address.call(calls[i]);
            require(success, "Function call failed");
            emit FunctionCalled(proxy_address, calls[i], result);
        }
    }

    /**
     * @dev Deploys a contract with the given init code and salt, using CREATE2.
     * @param init_code The initialization bytecode of the contract to deploy.
     * @param salt The salt for deploying the contract using CREATE2.
     * @return The address of the deployed contract.
     */
    function _deployContract(bytes memory init_code, bytes32 salt)
        private
        returns (address)
    {
        address deployed_address;

        assembly {
            deployed_address := create2(
                0,
                add(init_code, 0x20),
                mload(init_code),
                salt
            )
        }

        require(deployed_address != address(0), "Contract deployment failed");
        return deployed_address;
    }
}
