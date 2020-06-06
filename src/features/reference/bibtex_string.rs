use crate::{
    features::{prelude::*, reference::ReferenceContext},
    syntax::bibtex::Visitor,
};

pub fn find_bibtex_string_references(ctx: &mut ReferenceContext) {
    if let DocumentContent::Bibtex(tree) = &ctx.inner.current().content {
        let pos = ctx.inner.params.text_document_position.position;
        if let Some(name) = find_name(tree, pos) {
            let uri: Url = ctx.inner.current().uri.clone().into();
            if ctx.inner.params.context.include_declaration {
                for string in tree
                    .children(tree.root)
                    .filter_map(|node| tree.as_string(node))
                    .filter_map(|string| string.name.as_ref())
                    .filter(|string| string.text() == name.text())
                {
                    ctx.items.push(Location::new(uri.clone(), string.range()));
                }
            }

            let mut visitor = BibtexStringReferenceVisitor::default();
            visitor.visit(tree, tree.root);
            for reference in visitor
                .refs
                .into_iter()
                .filter(|reference| reference.text() == name.text())
            {
                ctx.items
                    .push(Location::new(uri.clone(), reference.range()));
            }
        }
    }
}

fn find_name(tree: &bibtex::Tree, pos: Position) -> Option<&bibtex::Token> {
    let mut nodes = tree.find(pos);
    nodes.reverse();
    let node0 = &tree.graph[nodes[0]];
    let node1 = nodes.get(1).map(|node| &tree.graph[*node]);
    match (node0, node1) {
        (bibtex::Node::Word(word), Some(bibtex::Node::Field(_)))
        | (bibtex::Node::Word(word), Some(bibtex::Node::Concat(_))) => Some(&word.token),
        (bibtex::Node::String(string), _) => string
            .name
            .as_ref()
            .filter(|name| name.range().contains(pos)),
        _ => None,
    }
}

#[derive(Debug, Default)]
pub struct BibtexStringReferenceVisitor<'a> {
    refs: Vec<&'a bibtex::Token>,
}

impl<'a> bibtex::Visitor<'a> for BibtexStringReferenceVisitor<'a> {
    fn visit(&mut self, tree: &'a bibtex::Tree, node: NodeIndex) {
        match &tree.graph[node] {
            bibtex::Node::Root(_)
            | bibtex::Node::Comment(_)
            | bibtex::Node::Preamble(_)
            | bibtex::Node::String(_)
            | bibtex::Node::Entry(_)
            | bibtex::Node::Word(_)
            | bibtex::Node::Command(_)
            | bibtex::Node::QuotedContent(_)
            | bibtex::Node::BracedContent(_) => (),
            bibtex::Node::Field(_) => {
                if let Some(word) = tree
                    .children(node)
                    .next()
                    .and_then(|content| tree.as_word(content))
                {
                    self.refs.push(&word.token);
                }
            }
            bibtex::Node::Concat(_) => {
                let mut children = tree.children(node);
                if let Some(word) = children.next().and_then(|left| tree.as_word(left)) {
                    self.refs.push(&word.token);
                }

                if let Some(word) = children.next().and_then(|right| tree.as_word(right)) {
                    self.refs.push(&word.token);
                }
            }
        }
        tree.walk(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;
    use indoc::indoc;

    #[test]
    fn definition() {
        let tester = FeatureTester::builder()
            .files(vec![(
                "main.bib",
                indoc!(
                    r#"
                        @string{foo = {Foo}}
                        @string{bar = {Bar}}
                        @article{baz, author = foo}
                    "#
                ),
            )])
            .main("main.bib")
            .line(2)
            .character(24)
            .build();
        let uri = tester.uri("main.bib");
        let mut ctx = ReferenceContext::new(tester.reference());

        find_bibtex_string_references(&mut ctx);

        let expected_items = vec![Location::new(uri.into(), Range::new_simple(2, 23, 2, 26))];
        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn definition_include_declaration() {
        let tester = FeatureTester::builder()
            .files(vec![(
                "main.bib",
                indoc!(
                    r#"
                        @string{foo = {Foo}}
                        @string{bar = {Bar}}
                        @article{baz, author = foo}
                    "#
                ),
            )])
            .main("main.bib")
            .line(2)
            .character(24)
            .include_declaration(true)
            .build();
        let uri = tester.uri("main.bib");
        let mut ctx = ReferenceContext::new(tester.reference());

        find_bibtex_string_references(&mut ctx);

        let expected_items = vec![
            Location::new(uri.clone().into(), Range::new_simple(0, 8, 0, 11)),
            Location::new(uri.into(), Range::new_simple(2, 23, 2, 26)),
        ];
        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn reference() {
        let tester = FeatureTester::builder()
            .files(vec![(
                "main.bib",
                indoc!(
                    r#"
                        @string{foo = {Foo}}
                        @string{bar = {Bar}}
                        @article{baz, author = foo}
                    "#
                ),
            )])
            .main("main.bib")
            .line(0)
            .character(10)
            .build();
        let uri = tester.uri("main.bib");
        let mut ctx = ReferenceContext::new(tester.reference());

        find_bibtex_string_references(&mut ctx);

        let expected_items = vec![Location::new(uri.into(), Range::new_simple(2, 23, 2, 26))];
        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn reference_include_declaration() {
        let tester = FeatureTester::builder()
            .files(vec![(
                "main.bib",
                indoc!(
                    r#"
                        @string{foo = {Foo}}
                        @string{bar = {Bar}}
                        @article{baz, author = foo}
                    "#
                ),
            )])
            .main("main.bib")
            .line(0)
            .character(10)
            .include_declaration(true)
            .build();
        let uri = tester.uri("main.bib");
        let mut ctx = ReferenceContext::new(tester.reference());

        find_bibtex_string_references(&mut ctx);

        let expected_items = vec![
            Location::new(uri.clone().into(), Range::new_simple(0, 8, 0, 11)),
            Location::new(uri.into(), Range::new_simple(2, 23, 2, 26)),
        ];
        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn empty_latex_document() {
        let mut ctx = ReferenceContext::new(
            FeatureTester::builder()
                .files(vec![("main.tex", "")])
                .main("main.tex")
                .line(0)
                .character(0)
                .build()
                .reference(),
        );

        find_bibtex_string_references(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn empty_bibtex_document() {
        let mut ctx = ReferenceContext::new(
            FeatureTester::builder()
                .files(vec![("main.bib", "")])
                .main("main.bib")
                .line(0)
                .character(0)
                .build()
                .reference(),
        );

        find_bibtex_string_references(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
