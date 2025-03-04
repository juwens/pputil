import * as fs from "fs";

// https://github.com/actions/github-script?tab=readme-ov-file
import * as core from '@actions/core';

// https://github.com/octokit/rest.js/
// https://octokit.github.io/rest.js/v21/
import { Octokit } from "@octokit/rest";


// https://github.com/octokit/octokit.js/
// import { Octokit, App } from "octokit";

const octokit = new Octokit({ auth: 'TODO' });

core.debug({
  'steps.create_release.outputs.release_id': '${{ steps.create_release.outputs.release_id }}',
  'core.getInput("release_id");': core.getInput('release_id', { required: false }),
  '{env.release_id }': '${{ env.release_id }}'
});

const releaseId = '${{ env.release_id }}';
const fileBlob = await fs.promises.readFile('artifact/pputil');

octokit.rest.repos.uploadReleaseAsset({
  owner: 'juwens', //context.repo.owner,
  repo: 'pputil', //context.repo.repo,
  release_id: releaseId,
  name: 'pputil.arm64',
  data: fileBlob,
});
