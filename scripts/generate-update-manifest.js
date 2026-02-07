#!/usr/bin/env node
/**
 * Generate Tauri update manifest for GitHub Releases
 * This script creates a latest.json file compatible with Tauri's updater
 *
 * Usage: VERSION=x.x.x node scripts/generate-update-manifest.js
 */

const fs = require('fs');
const path = require('path');

const version = process.env.VERSION;
const repoOwner = process.env.GITHUB_REPOSITORY_OWNER || 'screenpipe';
const repoName = process.env.GITHUB_REPOSITORY_NAME || 'screenpipe';

if (!version) {
  console.error('Error: VERSION environment variable is required');
  process.exit(1);
}

// Normalize version (strip 'v' prefix for consistent URL construction)
const normalizedVersion = version.startsWith('v') ? version.slice(1) : version;

// Platform mapping for Tauri updater
// Format: [tauri-target-arch]: [github-asset-name, signature-file-name]
const platforms = {
  'darwin-aarch64': { asset: 'aarch64-apple-darwin', sigExt: '.tar.gz.sig' },
  'darwin-x86_64': { asset: 'x86_64-apple-darwin', sigExt: '.tar.gz.sig' },
  'windows-x86_64': { asset: 'x86_64-pc-windows-msvc', sigExt: '.msi.zip.sig' },
  'linux-x86_64': { asset: 'x86_64-unknown-linux-gnu', sigExt: '.AppImage.tar.gz.sig' }
};

const manifest = {
  version: `v${normalizedVersion}`,
  notes: `Release v${normalizedVersion}`,
  pub_date: new Date().toISOString(),
  platforms: {}
};

const distDir = process.env.DIST_DIR || 'dist';

// Read signatures from built artifacts
for (const [platformKey, platformConfig] of Object.entries(platforms)) {
  try {
    // Try multiple signature file naming patterns
    const possibleSigFiles = [
      path.join(distDir, `${platformKey}.sig`),
      path.join(distDir, `screenpipe_${platformConfig.asset}${platformConfig.sigExt}`),
      path.join(distDir, `${platformConfig.asset}.sig`),
      path.join(distDir, `latest-${platformKey}.sig`)
    ];

    let sigContent = null;
    let usedFile = null;

    for (const sigFile of possibleSigFiles) {
      if (fs.existsSync(sigFile)) {
        sigContent = fs.readFileSync(sigFile, 'base64');
        usedFile = sigFile;
        break;
      }
    }

    if (!sigContent) {
      console.warn(`Warning: No signature file found for ${platformKey}, skipping...`);
      continue;
    }

    // Determine the download URL based on platform
    let downloadUrl;
    if (platformKey.startsWith('darwin')) {
      downloadUrl = `https://github.com/${repoOwner}/${repoName}/releases/download/v${normalizedVersion}/screenpipe_${platformConfig.asset}.tar.gz`;
    } else if (platformKey.startsWith('windows')) {
      downloadUrl = `https://github.com/${repoOwner}/${repoName}/releases/download/v${normalizedVersion}/screenpipe_${platformConfig.asset}.msi.zip`;
    } else {
      downloadUrl = `https://github.com/${repoOwner}/${repoName}/releases/download/v${normalizedVersion}/screenpipe_${platformConfig.asset}.AppImage.tar.gz`;
    }

    manifest.platforms[platformKey] = {
      signature: sigContent,
      url: downloadUrl
    };

    console.log(`✓ Added ${platformKey} (from ${usedFile})`);
  } catch (error) {
    console.error(`Error processing ${platformKey}:`, error.message);
  }
}

// Write manifest
const outputFile = process.env.OUTPUT_FILE || 'latest.json';
fs.writeFileSync(outputFile, JSON.stringify(manifest, null, 2));

console.log(`\n✓ Update manifest written to ${outputFile}`);
console.log(`  Version: ${manifest.version}`);
console.log(`  Platforms: ${Object.keys(manifest.platforms).join(', ')}`);
