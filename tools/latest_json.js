// Writes latest.json, what installed copies read at launch to find an update:
// node tools/latest_json.js VERSION TAG OWNER/REPO BUNDLE_DIR OUT_FILE
// Each installer goes with the signature the build made with the updater's private key; an MSI install
// updates from the MSI, everything else from the setup .exe.
const fs = require('fs'), path = require('path');
const [version, tag, repo, dir, out] = process.argv.slice(2);
const find = (sub, re) => {
  const d = path.join(dir, sub), f = fs.readdirSync(d).find(n => re.test(n));
  if (!f || !fs.existsSync(path.join(d, f + '.sig'))) throw new Error('no signed ' + sub + ' installer in ' + d);
  return { name: f, sig: fs.readFileSync(path.join(d, f + '.sig'), 'utf8').trim() };
};
const exe = find('nsis', /-setup\.exe$/), msi = find('msi', /\.msi$/);
const url = n => 'https://github.com/' + repo + '/releases/download/' + tag + '/' + encodeURIComponent(n);
const entry = f => ({ signature: f.sig, url: url(f.name) });
const doc = {
  version,
  notes: 'Heartwood ' + version,
  pub_date: new Date().toISOString(),
  platforms: { 'windows-x86_64': entry(exe), 'windows-x86_64-nsis': entry(exe), 'windows-x86_64-msi': entry(msi) },
};
fs.writeFileSync(out, JSON.stringify(doc, null, 2) + '\n');
console.log('latest.json for ' + version + ': ' + exe.name + ', ' + msi.name);
