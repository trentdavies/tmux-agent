#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitVersion {
    pub base_version: String,
    pub tag: Option<String>,
    pub commits_ahead: usize,
    pub hash: String,
    pub dirty: bool,
    pub exact: bool,
}

impl GitVersion {
    pub fn fallback(package_version: &str, commit_count: usize, hash: impl Into<String>) -> Self {
        Self {
            base_version: package_version.to_string(),
            tag: None,
            commits_ahead: commit_count,
            hash: hash.into(),
            dirty: false,
            exact: false,
        }
    }
}

pub fn parse_git_describe(
    describe: &str,
    package_version: &str,
    commit_count: usize,
) -> Option<GitVersion> {
    let dirty = describe.ends_with("-dirty");
    let describe = describe.strip_suffix("-dirty").unwrap_or(describe);

    if let Some((left, hash)) = describe.rsplit_once("-g") {
        if let Some((tag, commits_ahead)) = left.rsplit_once('-') {
            let commits_ahead = commits_ahead.parse::<usize>().ok()?;

            if is_release_tag(tag) {
                return Some(GitVersion {
                    base_version: tag.trim_start_matches('v').to_string(),
                    tag: Some(tag.to_string()),
                    commits_ahead,
                    hash: hash.to_string(),
                    dirty,
                    exact: commits_ahead == 0,
                });
            }

            return Some(GitVersion {
                base_version: package_version.to_string(),
                tag: None,
                commits_ahead,
                hash: hash.to_string(),
                dirty,
                exact: false,
            });
        }
    }

    Some(GitVersion {
        base_version: package_version.to_string(),
        tag: None,
        commits_ahead: commit_count,
        hash: describe.to_string(),
        dirty,
        exact: false,
    })
}

pub fn format_version(version: &GitVersion) -> String {
    if version.exact && !version.dirty {
        return version.base_version.clone();
    }

    let mut rendered = format!(
        "{}-dev.{}+g{}",
        version.base_version, version.commits_ahead, version.hash
    );

    if version.dirty {
        rendered.push_str(".dirty");
    }

    rendered
}

pub fn format_long_version(version: &GitVersion, rendered: &str) -> String {
    let source = version.tag.as_deref().unwrap_or("no matching tag");

    format!(
        "{rendered} (source: {source}, commits ahead: {}, commit: {}, dirty: {})",
        version.commits_ahead, version.hash, version.dirty
    )
}

fn is_release_tag(tag: &str) -> bool {
    let Some(version) = tag.strip_prefix('v') else {
        return false;
    };

    version.split('.').count() == 3
        && version
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::{format_long_version, format_version, parse_git_describe};

    #[test]
    fn exact_tag_stays_clean_semver() {
        let version = parse_git_describe("v1.2.3-0-gabc1234", "0.1.0", 10).unwrap();

        assert_eq!(format_version(&version), "1.2.3");
        assert_eq!(
            format_long_version(&version, "1.2.3"),
            "1.2.3 (source: v1.2.3, commits ahead: 0, commit: abc1234, dirty: false)"
        );
    }

    #[test]
    fn commits_ahead_use_dev_prerelease_and_build_metadata() {
        let version = parse_git_describe("v1.2.3-4-gabc1234", "0.1.0", 10).unwrap();

        assert_eq!(format_version(&version), "1.2.3-dev.4+gabc1234");
    }

    #[test]
    fn dirty_exact_tag_is_not_reported_as_release() {
        let version = parse_git_describe("v1.2.3-0-gabc1234-dirty", "0.1.0", 10).unwrap();

        assert_eq!(format_version(&version), "1.2.3-dev.0+gabc1234.dirty");
    }

    #[test]
    fn fallback_without_matching_tag_uses_package_version() {
        let version = parse_git_describe("abc1234-dirty", "0.4.0", 12).unwrap();

        assert_eq!(format_version(&version), "0.4.0-dev.12+gabc1234.dirty");
    }
}
