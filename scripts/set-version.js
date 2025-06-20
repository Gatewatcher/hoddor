const fs = require("fs");
const path = require("path");

const version = process.argv[2];
if (!version) {
  console.error("Missing version argument");
  process.exit(1);
}

const cargoPath = path.join(__dirname, "../hoddor/Cargo.toml");
const content = fs.readFileSync(cargoPath, "utf8");

const newContent = content.replace(
  /^version\s*=\s*".*?"$/m,
  `version = "${version}"`
);

fs.writeFileSync(cargoPath, newContent);

const packageJsonPath = path.join(__dirname, "../package.json");
const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
packageJson.version = version;
fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2) + "\n");

console.log(`✅ Updated Cargo.toml and package.json to version ${version}`);
