@_usage:
	just -l

# Tag & push
release:
    cargo workspaces version -a --force '*' --tag-prefix '' --no-individual-tags

# Publish on private 'cargo-hosted' registry
publish: 
    cargo publish --registry cargo-hosted