// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

/// Some docs
contract SimpleStorage {

  struct People {
    string name;
    uint256 favoriteNumber;
  }

  mapping(string => uint256) public nameToFavoriteNumber;

  uint256 favoriteNumber;

  People[] public people;

  function store(uint256 _favoriteNumber) public virtual {
    favoriteNumber = _favoriteNumber;
  }

  function retrieve() public view returns (uint256) {
    return favoriteNumber;
  }

  function addPerson(string memory _name, uint256 _favoriteNumber) public {
    people.push(People({favoriteNumber: _favoriteNumber, name: _name}));
    nameToFavoriteNumber[_name] = _favoriteNumber;
  }

  function removeLastPerson() public {
    people.pop();
  }
}