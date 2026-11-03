// Writes latest.json, what installed copies read at launch to find an update:
// node tools/latest_json.js VERSION TAG OWNER/REPO DIR OUT_FILE
// DIR holds both builds' signed files side by side, each with the signature the build made with the updater's
// private key. On Windows an MSI install updates from the MSI, everything else from the setup .exe; every Mac
// (Apple silicon or Intel) from the one universal app.
const fs = require('fs'), path = require('path');
const [version, tag, repo, dir, out] = process.argv.slice(2);
const find = (what, re) => {
  const f = fs.readdirSync(dir).find(n => re.test(n));
  if (!f || !fs.existsSync(path.join(dir, f + '.sig'))) throw new Error('no signed ' + what + ' in ' + dir);
  return { name: f, sig: fs.readFileSync(path.join(dir, f + '.sig'), 'utf8').trim() };
};
const exe = find('setup .exe', /_x64-setup\.exe$/), msi = find('.msi', /_x64_.*\.msi$/), mac = find('Mac app', /\.app\.tar\.gz$/);
const url = n => 'https://github.com/' + repo + '/releases/download/' + tag + '/' + encodeURIComponent(n);
const entry = f => ({ signature: f.sig, url: url(f.name) });
const doc = {
  version,
  notes: 'Heartwood ' + version,
  pub_date: new Date().toISOString(),
  platforms: {
    'windows-x86_64': entry(exe), 'windows-x86_64-nsis': entry(exe), 'windows-x86_64-msi': entry(msi),
    'darwin-aarch64': entry(mac), 'darwin-x86_64': entry(mac), 'darwin-aarch64-app': entry(mac), 'darwin-x86_64-app': entry(mac),
  },
};
fs.writeFileSync(out, JSON.stringify(doc, null, 2) + '\n');
console.log('latest.json for ' + version + ': ' + exe.name + ', ' + msi.name + ', ' + mac.name);
