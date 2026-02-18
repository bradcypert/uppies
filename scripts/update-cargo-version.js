#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

// Get version from command line argument
const newVersion = process.argv[2];

if (!newVersion) {
  console.error('Error: Version argument required');
  process.exit(1);
}

// Read Cargo.toml
const cargoPath = path.join(__dirname, '..', 'Cargo.toml');
let content = fs.readFileSync(cargoPath, 'utf8');

// Update version field (top level)
content = content.replace(
  /^version = ".*?"/m,
  `version = "${newVersion}"`
);

// Write back
fs.writeFileSync(cargoPath, content, 'utf8');

console.log(`Updated Cargo.toml version to ${newVersion}`);
