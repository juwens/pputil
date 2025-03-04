import * as fs from 'fs';
import * as core from '@actions/core'; // https://github.com/actions/github-script?tab=readme-ov-file
import { Octokit } from '@octokit/rest'; // https://github.com/octokit/rest.js/
// more relvant imports: https://github.com/actions/github-script?tab=readme-ov-file#actionsgithub-script

async function run(gh_token:string) {
  const octokit = new Octokit({ auth: gh_token });
  
  core.debug(JSON.stringify({
    'steps.create_release.outputs.release_id': '${{ steps.create_release.outputs.release_id }}',
    'core.getInput("release_id");': core.getInput('release_id', { required: false }),
    '{env.release_id }': '${{ env.release_id }}'
  }));
  
  const releaseId = Number.parseInt('${{ env.release_id }}');
  const fileBlob = await fs.promises.readFile('artifact/pputil');
  
  octokit.rest.repos.uploadReleaseAsset({
    owner: 'juwens', //context.repo.owner,
    repo: 'pputil', //context.repo.repo,
    release_id: releaseId,
    name: 'pputil.arm64',
    data: fileBlob as any,
  });
}

module.exports = {
  run
}