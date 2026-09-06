/// 綴じ方向（見開き時の左右配置）
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Binding {
    /// 左綴じ（横書き・洋書）: 小さいページ番号が左
    LeftToRight,
    /// 右綴じ（縦書き・和書）: 小さいページ番号が右
    RightToLeft,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ViewMode {
    Single,
    Spread,
}

/// 指定ページを含む見開きの先頭(最小)ページ番号を返す。
/// 表紙単独時は [0] [1,2] [3,4]… 、それ以外は [0,1] [2,3]… で組む。
pub(crate) fn spread_start(view_mode: ViewMode, cover_alone: bool, page: usize) -> usize {
    match view_mode {
        ViewMode::Single => page,
        ViewMode::Spread => {
            if cover_alone {
                if page == 0 { 0 } else { ((page - 1) & !1) + 1 }
            } else {
                page & !1
            }
        }
    }
}

/// 見開き先頭 `start` の見開きに含まれるページ番号を昇順で返す。
pub(crate) fn pages_of_spread(
    view_mode: ViewMode,
    cover_alone: bool,
    page_count: usize,
    start: usize,
) -> Vec<usize> {
    if page_count == 0 {
        return vec![];
    }
    let start = start.min(page_count - 1);
    match view_mode {
        ViewMode::Single => vec![start],
        ViewMode::Spread => {
            if cover_alone && start == 0 {
                return vec![0];
            }
            let mut pages = vec![start];
            if start + 1 < page_count {
                pages.push(start + 1);
            }
            pages
        }
    }
}

/// 現在の見開きに含まれるページ番号を昇順で返す。
pub(crate) fn current_pages_sorted(
    view_mode: ViewMode,
    cover_alone: bool,
    page_count: usize,
    current: usize,
) -> Vec<usize> {
    pages_of_spread(
        view_mode,
        cover_alone,
        page_count,
        spread_start(view_mode, cover_alone, current),
    )
}

/// 前後の見開きに含まれる先読み対象ページを返す。
pub(crate) fn prefetch_targets(
    view_mode: ViewMode,
    cover_alone: bool,
    page_count: usize,
    current: usize,
) -> Vec<usize> {
    let cur = current_pages_sorted(view_mode, cover_alone, page_count, current);
    let mut targets = Vec::new();
    if let Some(&last) = cur.last() {
        if last + 1 < page_count {
            targets.extend(pages_of_spread(
                view_mode,
                cover_alone,
                page_count,
                spread_start(view_mode, cover_alone, last + 1),
            ));
        }
    }
    if let Some(&first) = cur.first() {
        if first > 0 {
            targets.extend(pages_of_spread(
                view_mode,
                cover_alone,
                page_count,
                spread_start(view_mode, cover_alone, first - 1),
            ));
        }
    }
    targets
}

/// 現在表示すべきページ番号を左→右の表示順で返す。
pub(crate) fn visible_pages(
    view_mode: ViewMode,
    binding: Binding,
    cover_alone: bool,
    page_count: usize,
    current: usize,
) -> Vec<usize> {
    let mut pages = current_pages_sorted(view_mode, cover_alone, page_count, current);
    if binding == Binding::RightToLeft && pages.len() == 2 {
        pages.reverse();
    }
    pages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cover_alone_groups_cover_and_following_spreads() {
        assert_eq!(spread_start(ViewMode::Spread, true, 0), 0);
        assert_eq!(spread_start(ViewMode::Spread, true, 1), 1);
        assert_eq!(spread_start(ViewMode::Spread, true, 2), 1);
        assert_eq!(spread_start(ViewMode::Spread, true, 3), 3);
        assert_eq!(pages_of_spread(ViewMode::Spread, true, 5, 0), vec![0]);
        assert_eq!(pages_of_spread(ViewMode::Spread, true, 5, 1), vec![1, 2]);
    }

    #[test]
    fn spread_without_cover_alone_groups_from_zero() {
        assert_eq!(spread_start(ViewMode::Spread, false, 0), 0);
        assert_eq!(spread_start(ViewMode::Spread, false, 1), 0);
        assert_eq!(spread_start(ViewMode::Spread, false, 2), 2);
        assert_eq!(pages_of_spread(ViewMode::Spread, false, 3, 2), vec![2]);
    }

    #[test]
    fn single_mode_keeps_requested_page() {
        assert_eq!(spread_start(ViewMode::Single, true, 3), 3);
        assert_eq!(pages_of_spread(ViewMode::Single, true, 5, 3), vec![3]);
    }

    #[test]
    fn right_binding_reverses_only_two_page_spreads() {
        assert_eq!(
            visible_pages(ViewMode::Spread, Binding::RightToLeft, true, 5, 1),
            vec![2, 1]
        );
        assert_eq!(
            visible_pages(ViewMode::Spread, Binding::RightToLeft, true, 5, 0),
            vec![0]
        );
        assert_eq!(
            visible_pages(ViewMode::Spread, Binding::LeftToRight, true, 5, 1),
            vec![1, 2]
        );
    }

    #[test]
    fn prefetches_adjacent_spreads() {
        assert_eq!(
            prefetch_targets(ViewMode::Spread, true, 7, 3),
            vec![5, 6, 1, 2]
        );
        assert_eq!(prefetch_targets(ViewMode::Spread, true, 7, 0), vec![1, 2]);
    }
}
