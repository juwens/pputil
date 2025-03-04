import * as fs from 'fs';
import * as core from '@actions/core'; // https://github.com/actions/github-script?tab=readme-ov-file
import { Octokit } from '@octokit/rest'; // https://github.com/octokit/rest.js/
import { Context } from '@actions/github/lib/context.js';

// more relvant imports:
// https://github.com/actions/toolkit
// https://github.com/actions/github-script?tab=readme-ov-file#actionsgithub-script

export default async function run({octokit, context} : {octokit : Octokit, context : Context} ) {
  core.debug(JSON.stringify({
    'steps.create_release.outputs.release_id': '${{ steps.create_release.outputs.release_id }}',
    'core.getInput("release_id");': core.getInput('release_id', { required: false }),
    '{env.release_id }': '${{ env.release_id }}'
  }));

  const releaseId = Number.parseInt('${{ env.release_id }}');
  const fileBlob = await fs.promises.readFile('artifact/pputil');

  octokit.rest.repos.uploadReleaseAsset({
    owner: context.repo.owner, //,
    repo: context.repo.repo,
    release_id: releaseId,
    name: 'pputil.arm64',
    data: fileBlob as any,
  });
}