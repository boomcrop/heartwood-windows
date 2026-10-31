// Sets the app's version everywhere it's written down: node tools/set_version.js 1.2.3
// (the release workflow takes it from the tag, so a tag is all a release needs)
const fs = require('fs'), path = require('path'), root = path.join(__dirname, '..');
const v = String(process.argv[2] || '').replace(/^v/, '');
if (!/^\d+\.\d+\.\d+$/.test(v)) { console.error('a version like 1.2.3, please (got "' + process.argv[2] + '")'); process.exit(1); }
const json = (f, edit) => { const p = path.join(root, f), o = JSON.parse(fs.readFileSync(p, 'utf8')); edit(o); fs.writeFileSync(p, JSON.stringify(o, null, 2) + '\n'); };
json('src-tauri/tauri.conf.json', o => { o.version = v; });
json('package.json', o => { o.version = v; });
const cargo = path.join(root, 'src-tauri/Cargo.toml');
fs.writeFileSync(cargo, fs.readFileSync(cargo, 'utf8').replace(/^(\[package\][\s\S]*?\nversion = )"[^"]*"/, '$1"' + v + '"'));
console.log('version ' + v);
