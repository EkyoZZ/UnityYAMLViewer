const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const source=fs.readFileSync(__dirname+'/main.js','utf8'),html=fs.readFileSync(__dirname+'/index.html','utf8');
assert(!html.includes('id="path-grid"'));assert(!source.includes('async function createGridPath('));
const host={};vm.runInNewContext(source.slice(source.indexOf('function gridPaths('),source.indexOf('function atlasSpec(')),host);
const legacy={name:'辅助网格路径 [UYTile] 2x4 128x64'};const found=host.gridPaths({pathItems:[{name:'用户路径'},legacy]});assert.equal(found.length,1);assert.equal(found[0].path,legacy);assert.equal(found[0].rows,4);assert.equal(found[0].tw,128);
console.log('Path rollback checks passed: no creation toggle or creator; legacy path metadata stays readable without modifying paths.');
