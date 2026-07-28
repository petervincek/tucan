# Define the target: build (will download the dependencies and compile the project)
build:
	cargo build

# Define the target: build-prod
build-prod:
	cargo build --release

# Define the target: clean
clean:
	cargo clean

# Define the target: update
update:
	cargo update

# Define the target: test
test:
	cargo test

# Define the target: check
check:
	cargo check

# Define the target: lint
lint:
	cargo clippy

# Define the target: lint-fix
lint-fix:
	cargo clippy --fix

# Define the target: run
run:
	cargo run

# Define the target: run-release (will compile while eliminating the debug statements)
run-release:
	cargo run --release

# Define the target: cloc
cloc:
	cloc --exclude-dir=target  .

# Define the target: tag (create and push an annotated release tag)
# Usage:
#   make tag VERSION=v0.1.0
# Optional:
#   make tag VERSION=v0.1.0 BRANCH=master
tag:
	@if [ -z "$(VERSION)" ]; then \
		echo "ERROR: VERSION is required. Example: make tag VERSION=v0.1.0"; \
		exit 1; \
	fi
	@if ! echo "$(VERSION)" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$$'; then \
		echo "ERROR: VERSION must look like vX.Y.Z or vX.Y.Z-rc.1"; \
		exit 1; \
	fi
	@if [ -n "$$(git status --porcelain)" ]; then \
		echo "ERROR: Working tree is dirty. Commit or stash changes first."; \
		exit 1; \
	fi
	@TARGET_BRANCH="$(if $(BRANCH),$(BRANCH),main)"; \
		CURRENT_BRANCH="$$(git rev-parse --abbrev-ref HEAD)"; \
		if [ "$$CURRENT_BRANCH" != "$$TARGET_BRANCH" ]; then \
			echo "ERROR: You are on '$$CURRENT_BRANCH'. Switch to '$$TARGET_BRANCH' first."; \
			exit 1; \
		fi
	@git fetch origin --prune
	@git pull --ff-only origin $(if $(BRANCH),$(BRANCH),main)
	@if git rev-parse "$(VERSION)" >/dev/null 2>&1; then \
		echo "ERROR: Tag $(VERSION) already exists locally."; \
		exit 1; \
	fi
	@git tag -a "$(VERSION)" -m "Release $(VERSION)"
	@git push origin "$(VERSION)"
	@echo "Created and pushed tag $(VERSION)"

# Define the target: untag (delete local+remote tag safely)
# Usage:
#   make untag VERSION=v0.1.0 CONFIRM=YES
untag:
	@if [ -z "$(VERSION)" ]; then \
		echo "ERROR: VERSION is required. Example: make untag VERSION=v0.1.0 CONFIRM=YES"; \
		exit 1; \
	fi
	@if [ "$(CONFIRM)" != "YES" ]; then \
		echo "ERROR: Refusing to delete tag without CONFIRM=YES"; \
		exit 1; \
	fi
	@if git rev-parse "$(VERSION)" >/dev/null 2>&1; then \
		git tag -d "$(VERSION)"; \
	else \
		echo "INFO: Local tag $(VERSION) does not exist."; \
	fi
	@git push origin ":refs/tags/$(VERSION)" || true
	@echo "Deleted local/remote tag $(VERSION) (if it existed)."

# Define the target: release-delete (delete a GitHub release by tag)
# Requires GitHub CLI (gh) authenticated for this repository.
# Usage:
#   make release-delete VERSION=v0.1.0 CONFIRM=YES
# Optional: also delete tag from local+remote
#   make release-delete VERSION=v0.1.0 CONFIRM=YES DELETE_TAG=true
release-delete:
	@if [ -z "$(VERSION)" ]; then \
		echo "ERROR: VERSION is required. Example: make release-delete VERSION=v0.1.0 CONFIRM=YES"; \
		exit 1; \
	fi
	@if [ "$(CONFIRM)" != "YES" ]; then \
		echo "ERROR: Refusing to delete GitHub release without CONFIRM=YES"; \
		exit 1; \
	fi
	@if ! command -v gh >/dev/null 2>&1; then \
		echo "ERROR: gh CLI not found. Install GitHub CLI to manage releases from Makefile."; \
		exit 1; \
	fi
	@gh release delete "$(VERSION)" --yes || true
	@echo "Deleted GitHub release $(VERSION) (if it existed)."
	@if [ "$(DELETE_TAG)" = "true" ]; then \
		$(MAKE) untag VERSION="$(VERSION)" CONFIRM=YES; \
	fi