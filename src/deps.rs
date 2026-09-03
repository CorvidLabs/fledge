use anyhow::{bail, Result};
use std::collections::HashSet;

/// Depth-first walk of a task dependency graph.
///
/// Two sets keep diamond DAGs (two tasks sharing one dep) from being reported
/// as cycles:
///
/// - `in_progress` is the recursion stack. Insert before walking deps, remove
///   after returning. A hit here is a genuine cycle; the error lists that
///   ordered walk (not a `HashSet` iteration).
/// - `completed` is nodes whose subgraph has already been walked. A hit here
///   returns `Ok` immediately so shared deps are not re-run.
///
/// `on_ready` is invoked once per node after its dependencies, just before the
/// node is marked completed.
pub(crate) fn walk_task_graph<'graph, Deps, Ready>(
    name: &str,
    deps_of: &Deps,
    in_progress: &mut Vec<String>,
    completed: &mut HashSet<String>,
    on_ready: &mut Ready,
) -> Result<()>
where
    Deps: Fn(&str) -> Option<&'graph [String]>,
    Ready: FnMut(&str) -> Result<()>,
{
    if completed.contains(name) {
        return Ok(());
    }
    if let Some(idx) = in_progress.iter().position(|n| n == name) {
        let mut cycle: Vec<&str> = in_progress[idx..].iter().map(String::as_str).collect();
        cycle.push(name);
        bail!("Circular dependency detected: {}", cycle.join(" → "));
    }

    in_progress.push(name.to_string());
    for dep in deps_of(name).unwrap_or(&[]) {
        walk_task_graph(dep, deps_of, in_progress, completed, on_ready)?;
    }
    on_ready(name)?;
    in_progress.pop();
    completed.insert(name.to_string());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn graph(edges: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        edges
            .iter()
            .map(|(name, deps)| {
                (
                    (*name).to_string(),
                    deps.iter().map(|d| (*d).to_string()).collect(),
                )
            })
            .collect()
    }

    fn walk(g: &BTreeMap<String, Vec<String>>, start: &str) -> Result<Vec<String>> {
        let mut ready = Vec::new();
        walk_task_graph(
            start,
            &|n| g.get(n).map(Vec::as_slice),
            &mut Vec::new(),
            &mut HashSet::new(),
            &mut |n| {
                ready.push(n.to_string());
                Ok(())
            },
        )?;
        Ok(ready)
    }

    #[test]
    fn diamond_is_not_a_cycle_and_shared_dep_runs_once() {
        // a → [b, c], b → [d], c → [d]
        let g = graph(&[("a", &["b", "c"]), ("b", &["d"]), ("c", &["d"]), ("d", &[])]);
        let ready = walk(&g, "a").expect("diamond DAG must not be reported as a cycle");
        assert_eq!(ready, vec!["d", "b", "c", "a"]);
        assert_eq!(ready.iter().filter(|n| *n == "d").count(), 1);
    }

    #[test]
    fn two_cycle_reports_ordered_path() {
        let g = graph(&[("a", &["b"]), ("b", &["a"])]);
        let err = walk(&g, "a").unwrap_err().to_string();
        assert!(
            err.contains("Circular dependency detected: a → b → a"),
            "expected ordered cycle walk, got: {err}"
        );
    }

    #[test]
    fn self_cycle_reports_ordered_path() {
        let g = graph(&[("a", &["a"])]);
        let err = walk(&g, "a").unwrap_err().to_string();
        assert!(
            err.contains("Circular dependency detected: a → a"),
            "expected self-cycle walk, got: {err}"
        );
    }

    #[test]
    fn longer_cycle_reports_actual_cycle_not_prefix() {
        // x is on the path to the cycle but not on the cycle itself
        let g = graph(&[("x", &["a"]), ("a", &["b"]), ("b", &["c"]), ("c", &["a"])]);
        let err = walk(&g, "x").unwrap_err().to_string();
        assert!(
            err.contains("Circular dependency detected: a → b → c → a"),
            "expected the cycle itself as an ordered walk, got: {err}"
        );
        assert!(
            !err.contains("x →"),
            "cycle path must not include the acyclic prefix, got: {err}"
        );
    }

    #[test]
    fn detect_cycle_ok_on_diamond() {
        let g = graph(&[("a", &["b", "c"]), ("b", &["d"]), ("c", &["d"]), ("d", &[])]);
        walk(&g, "a").unwrap();
    }

    #[test]
    fn detect_cycle_err_on_loop() {
        let g = graph(&[("a", &["b"]), ("b", &["a"])]);
        let err = walk(&g, "a").unwrap_err().to_string();
        assert!(err.contains("a → b → a"), "got: {err}");
    }
}
