// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

/// Some docs
contract SimpleStorage {

  struct People {
    string name;
    uint16 favoriteNumber;
  }

  mapping(string => uint16) public nameToFavoriteNumber;

  uint16 favoriteNumber;

  People[] public people;

  function store(uint16 _favoriteNumber) public virtual {
    favoriteNumber = _favoriteNumber;
  }

  function retrieve() public view returns (uint16) {
    return favoriteNumber;
  }

  function addPerson(string memory _name, uint16 _favoriteNumber) public {
    people.push(People({favoriteNumber: _favoriteNumber, name: _name}));
    nameToFavoriteNumber[_name] = _favoriteNumber;
  }

  function removeLastPerson() public {
    people.pop();
  }
}