# Contributing to NSLauncher

We would love for you to contribute to NSLauncher and help make it even better than it is today!
As a contributor, here are the guidelines we would like you to follow:

 - [Issues and Bugs](#issue)
 - [Commit Message Format](#commit)
 
 ## <a name="issue"></a> Found a Bug?
 
 If you find a bug in the source code, you can help us by submitting an issue to our [GitHub Repository][github].
 Even better, you can submit a Pull Request with a fix.

## <a name="commit"></a> Commit Message Format

### Summary

The Conventional Commits specification is a lightweight convention on top of commit messages.
It provides an easy set of rules for creating an explicit commit history;
which makes it easier to write automated tools on top of.
This convention dovetails with [SemVer](http://semver.org),
by describing the features, fixes, and breaking changes made in commit messages.

The commit message should be structured as follows:

---

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```
---

<br />

The `<type>` and `<summary>` fields are mandatory, the `(<scope>)` field is optional.


#### Type

Must be one of the following:

* **build**: Changes that affect the build system or external dependencies (example scopes: cargo)
* **ci**: Changes to our CI configuration files and scripts (example scopes: Circle, BrowserStack, SauceLabs)
* **docs**: Documentation only changes
* **feat**: A new feature
* **fix**: A bug fix
* **perf**: A code change that improves performance
* **refactor**: A code change that neither fixes a bug nor adds a feature
* **test**: Adding missing tests or correcting existing tests
* **BREAKING CHANGE:** a commit that has a footer `BREAKING CHANGE:`, or appends a `!` after the type/scope, introduces a breaking API change (correlating with [`MAJOR`](http://semver.org/#summary) in semantic versioning).
  A BREAKING CHANGE can be part of commits of any _type_.
* **IMPORTANT CHANGE:** a commit that has a footer `IMPORTANT CHANGE:`, or appends a `$` after the type/scope, introduces a important API change (correlating with [`MAJOR`](http://semver.org/#summary) in semantic versioning).
  A IMPORTANT CHANGE can be part of commits of any _type_.

#### Scope
The scope should be the name of the launcher part affected (as perceived by the person reading the changelog generated from commit messages).

The following is the list of supported scopes:

* `client`
* `server`
* `api`
* `macro`


#### Revert commits

If the commit reverts a previous commit, it should begin with `revert: `, followed by the header of the reverted commit.

The content of the commit message body should contain:

- information about the SHA of the commit being reverted in the following format: `This reverts commit <SHA>`,
- a clear description of the reason for reverting the commit message.


[github]: https://github.com/team-ns/launcher