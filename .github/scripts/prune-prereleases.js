// Grouping function
function groupBy(array, keyOrIterator) {
    const iterator = typeof keyOrIterator === 'function' 
        ? keyOrIterator 
        : item => item[String(keyOrIterator)];

    return array.reduce((memo, item) => {
        const key = iterator(item);
        if (!memo[key]) {
            memo[key] = [];
        }
        memo[key].push(item);
        return memo;
    }, {});
}

module.exports = async ({ github, context }) => {
    try {
        console.log("Pruning old prereleases");

        // Fetching releases from GitHub
        const { data: releases } = await github.rest.repos.listReleases({
            owner: context.repo.owner,
            repo: context.repo.repo,
        });

        // Filter releases with 'nightly' tag
        const nightlies = releases.filter(
            release =>
                release.tag_name.includes("nightly") && release.tag_name !== "nightly"
        );

        // Group releases by year and month (YYYY-MM format)
        const groupedByMonth = groupBy(nightlies, release => release.created_at.slice(0, 7));

        // Apply pruning rules: 1. Keep the earliest release per month, 2. Keep the newest 3 nightlies
        const nightliesToPrune = Object.values(groupedByMonth)
            .reduce((acc, monthReleases) => {
                // Keep all but the most recent release per month
                acc.push(...monthReleases.slice(0, -1));
                return acc;
            }, [])
            .slice(3); // Keep only the newest 3 nightlies

        // Deleting releases and tags
        for (const nightly of nightliesToPrune) {
            console.log(`Deleting nightly: ${nightly.tag_name}`);

            // Delete release
            await github.rest.repos.deleteRelease({
                owner: context.repo.owner,
                repo: context.repo.repo,
                release_id: nightly.id,
            });

            // Delete release tag
            console.log(`Deleting nightly tag: ${nightly.tag_name}`);
            await github.rest.git.deleteRef({
                owner: context.repo.owner,
                repo: context.repo.repo,
                ref: `tags/${nightly.tag_name}`,
            });
        }

        console.log("Done.");
    } catch (error) {
        console.error("Error during pruning:", error);
    }
};
