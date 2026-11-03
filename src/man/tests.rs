// SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the manual pages, each of which names the configuration file
//! and the directory of unsaved changes of its own platform alone, and
//! mentions XDG and Wayland only where they apply

use strum::VariantArray;

use super::*;

/// Date that the pages give in the tests
const DATE: &str = "2026-11-01";

/// Writes a path as roff and scdoc both write it, with each backslash
/// doubled
fn escaped(path: &str) -> String {
    path.replace('\\', r"\\")
}

/// Returns the scdoc source of the page of `name` for `platform`
fn page(name: &str, platform: Platform) -> String {
    let (_, source) = SCDOC
        .iter()
        .find(|(page, _)| *page == name)
        .expect("the page is among those that scdoc renders");
    for_platform(source, platform)
}

#[test]
fn targets_go_by_where_dirs_finds_the_configuration() {
    assert_eq!(Platform::of("linux"), Platform::Xdg);
    assert_eq!(Platform::of("freebsd"), Platform::Xdg);
    assert_eq!(Platform::of("openbsd"), Platform::Xdg);
    assert_eq!(Platform::of("macos"), Platform::Macos);
    assert_eq!(Platform::of("windows"), Platform::Windows);
}

#[test]
fn pages_mention_xdg_only_where_it_applies() {
    Platform::VARIANTS.iter().for_each(|&platform| {
        let xdg = platform == Platform::Xdg;
        let command = command_page(platform, DATE);
        assert_eq!(command.contains("XDG"), xdg, "cinq.1 for {platform:?}");
        SCDOC.iter().for_each(|(name, _)| {
            let page = page(name, platform);
            assert_eq!(page.contains("XDG"), xdg, "{name} for {platform:?}");
        });
    });
}

#[test]
fn pages_name_the_files_of_their_platform_alone() {
    Platform::VARIANTS.iter().for_each(|&platform| {
        let command = command_page(platform, DATE);
        let configuration = [&command, &page("cinq.5", platform)];
        let unsaved = [&command, &page("cinq-show.5", platform)];
        Platform::VARIANTS.iter().for_each(|&other| {
            let config = escaped(other.config_file());
            let directory = escaped(other.unsaved_directory());
            assert!(
                configuration
                    .iter()
                    .all(|page| page.contains(&config) == (other == platform)),
                "pages for {platform:?} name the configuration file of {other:?} wrongly"
            );
            assert!(
                unsaved
                    .iter()
                    .all(|page| page.contains(&directory) == (other == platform)),
                "pages for {platform:?} name the unsaved changes of {other:?} wrongly"
            );
        });
    });
}

#[test]
fn command_page_names_wayland_only_where_the_window_needs_it() {
    let needs = |platform: Platform| command_page(platform, DATE).contains("WAYLAND_DISPLAY");
    assert!(needs(Platform::Xdg));
    assert!(!needs(Platform::Macos));
    assert!(!needs(Platform::Windows));
}

#[test]
fn source_marks_parts_for_platforms_alone() {
    let names: Vec<&str> = Platform::VARIANTS
        .iter()
        .map(|platform| platform.name())
        .collect();
    let marked: Vec<&str> = SCDOC
        .iter()
        .flat_map(|(_, source)| source.lines())
        .filter_map(|line| line.strip_prefix(IF))
        .flat_map(str::split_whitespace)
        .collect();
    assert!(
        marked.iter().all(|name| names.contains(name)),
        "the scdoc sources mark parts for {marked:?}, of which only {names:?} are platforms"
    );
}

#[test]
fn scdoc_pages_start_with_their_preamble() {
    Platform::VARIANTS.iter().for_each(|&platform| {
        SCDOC.iter().for_each(|(name, _)| {
            let preamble = name.replacen('.', "(", 1) + ") ";
            assert!(
                page(name, platform).starts_with(&preamble),
                "{name} for {platform:?}"
            );
        });
    });
}

#[test]
fn command_page_gives_its_date_and_version_in_their_places() {
    let title = format!(
        r#".TH cinq 1 {DATE} "cinq {}" "General Commands Manual""#,
        env!("CARGO_PKG_VERSION")
    );
    assert!(
        command_page(Platform::Xdg, DATE)
            .lines()
            .any(|line| line == title),
        "cinq(1) lacks the title {title}"
    );
}
