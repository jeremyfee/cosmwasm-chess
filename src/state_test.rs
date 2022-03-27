#[cfg(test)]
mod tests {
    use crate::state::merge_iters;

    #[test]
    fn test_merge_iters() {
        let merged = merge_iters(
            vec![1, 3, 5, 7].into_iter(),
            vec![2, 4, 6].into_iter(),
            |n1, n2| -> bool { n1 <= n2 },
        )
        .collect::<Vec<_>>();

        assert_eq!(merged, vec![1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn test_merge_iters_empty_first() {
        let merged = merge_iters(
            vec![].into_iter(),
            vec![2, 4, 6].into_iter(),
            |n1, n2| -> bool { n1 <= n2 },
        )
        .collect::<Vec<_>>();

        assert_eq!(merged, vec![2, 4, 6]);
    }

    #[test]
    fn test_merge_iters_empty_second() {
        let merged = merge_iters(
            vec![1, 3, 5].into_iter(),
            vec![].into_iter(),
            |n1, n2| -> bool { n1 <= n2 },
        )
        .collect::<Vec<_>>();

        assert_eq!(merged, vec![1, 3, 5]);
    }
}
