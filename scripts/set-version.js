const fs = require("fs");
const path = require("path");

const version = process.env.npm_package_version;
if (!version) {
  console.error("Missing version from npm");
  process.exit(1);
}

const cargoPath = path.join(__dirname, "../hoddor/Cargo.toml");
const content = fs.readFileSync(cargoPath, "utf8");

const newContent = content.replace(
  /^version\s*=\s*".*?"$/m,
  `version = "${version}"`
);

fs.writeFileSync(cargoPath, newContent);
console.log(`🔧 Updated Cargo.toml to version ${version}`);
