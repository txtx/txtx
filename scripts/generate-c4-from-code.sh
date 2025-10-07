#!/usr/bin/env bash
# Generate Structurizr DSL from C4 annotations in Rust code

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT_FILE="$PROJECT_ROOT/docs/architecture/linter/workspace.dsl"

echo "🔍 Scanning for C4 annotations in Rust code..."

# Find all Rust files with C4 annotations
files=$(grep -r "@c4-" "$PROJECT_ROOT/crates" --include="*.rs" -l | sort)

if [ -z "$files" ]; then
    echo "❌ No C4 annotations found"
    exit 1
fi

echo "✓ Found annotations in:"
echo "$files" | sed 's/^/  - /'
echo

# Extract annotations
declare -A components
declare -A containers
declare -A relationships
declare -A responsibilities

# Save files to temp file to avoid nested process substitution issues
tmpfile=$(mktemp)
echo "$files" > "$tmpfile"

while IFS= read -r file; do
    echo "  Processing: $file" >&2
    # Extract component info (strip comment markers //!, ///, //)
    component=$(grep -h "@c4-component" "$file" | sed 's|.*@c4-component \(.*\)|\1|' | sed 's/^[ \t]*//' | head -1)
    container=$(grep -h "@c4-container" "$file" | sed 's|.*@c4-container \(.*\)|\1|' | sed 's/^[ \t]*//' | head -1)
    description=$(grep -h "@c4-description" "$file" | sed 's|.*@c4-description \(.*\)|\1|' | sed 's/^[ \t]*//' | head -1)
    technology=$(grep -h "@c4-technology" "$file" | sed 's|.*@c4-technology \(.*\)|\1|' | sed 's/^[ \t]*//' | head -1)

    if [ -n "$component" ]; then
        echo "    Component: $component" >&2
        key="${component}|${container}|${description}|${technology}"
        components["$key"]=1

        # Extract relationships
        grep -h "@c4-relationship" "$file" | sed 's/.*@c4-relationship "\([^"]*\)" "\([^"]*\)"/\1|\2/' | while IFS= read -r rel; do
            relationships["${component}|${rel}"]=1
        done || true

        # Extract uses relationships
        grep -h "@c4-uses" "$file" | while IFS= read -r uses; do
            target=$(echo "$uses" | sed 's/.*@c4-uses \([^ ]*\).*/\1/')
            desc=$(echo "$uses" | sed 's/.*@c4-uses [^ ]* "\(.*\)"/\1/')
            relationships["${component}|uses|${target}|${desc}"]=1
        done || true

        # Extract responsibilities
        grep -h "@c4-responsibility" "$file" | sed 's|.*@c4-responsibility \(.*\)|\1|' | sed 's/^[ \t]*//' | while IFS= read -r resp; do
            responsibilities["${component}|${resp}"]=1
        done || true
    fi
done < "$tmpfile"

rm -f "$tmpfile"

# Generate Structurizr DSL
echo "📝 Generating Structurizr DSL..." >&2
echo "  Found ${#components[@]} components" >&2

cat > "$OUTPUT_FILE" <<'EOF'
workspace "txtx Linter Architecture (Generated from Code)" "Auto-generated from C4 annotations in Rust source" {

    model {
        user = person "Developer" "Writes txtx runbooks and manifests"

        txtxSystem = softwareSystem "txtx CLI" "Command-line tool for runbook execution and validation" {
EOF

# Group components by container
declare -A container_components

for key in "${!components[@]}"; do
    IFS='|' read -r component container description technology <<< "$key"
    if [ -n "$container" ]; then
        container_components["$container"]+="${component}|${description}|${technology}"$'\n'
    fi
done

# Generate containers and components
for container in "${!container_components[@]}"; do
    # Sanitize container name for DSL
    container_id=$(echo "$container" | tr '[:upper:] ' '[:lower:]_')

    cat >> "$OUTPUT_FILE" <<EOF

            ${container_id} = container "$container" "Container for $container components" "Rust" {
EOF

    # Add components to this container
    while IFS= read -r comp_line; do
        [ -z "$comp_line" ] && continue
        IFS='|' read -r component description technology <<< "$comp_line"
        component_id=$(echo "$component" | tr '[:upper:] ' '[:lower:]_')

        cat >> "$OUTPUT_FILE" <<EOF
                ${component_id} = component "$component" "$description" "$technology"
EOF

        # Add responsibilities as notes
        for resp_key in "${!responsibilities[@]}"; do
            IFS='|' read -r resp_comp resp <<< "$resp_key"
            if [ "$resp_comp" = "$component" ]; then
                echo "                // Responsibility: $resp" >> "$OUTPUT_FILE"
            fi
        done

    done <<< "${container_components[$container]}"

    echo "            }" >> "$OUTPUT_FILE"
done

cat >> "$OUTPUT_FILE" <<'EOF'
        }

        // Relationships
EOF

# Add relationships
for rel_key in "${!relationships[@]}"; do
    IFS='|' read -r source rel_type target desc <<< "$rel_key"
    source_id=$(echo "$source" | tr '[:upper:] ' '[:lower:]_')

    if [ "$rel_type" = "uses" ]; then
        target_id=$(echo "$target" | tr '[:upper:] ' '[:lower:]_')
        echo "        ${source_id} -> ${target_id} \"${desc}\"" >> "$OUTPUT_FILE"
    elif [ -n "$target" ]; then
        target_id=$(echo "$target" | tr '[:upper:] ' '[:lower:]_')
        echo "        ${source_id} -> ${target_id} \"${rel_type}\"" >> "$OUTPUT_FILE"
    fi
done

cat >> "$OUTPUT_FILE" <<'EOF'
    }

    views {
        systemContext txtxSystem "SystemContext" {
            include *
            autoLayout lr
        }

EOF

# Generate container views
for container in "${!container_components[@]}"; do
    container_id=$(echo "$container" | tr '[:upper:] ' '[:lower:]_')
    cat >> "$OUTPUT_FILE" <<EOF
        component ${container_id} "${container}" {
            include *
            autoLayout tb
        }

EOF
done

cat >> "$OUTPUT_FILE" <<'EOF'
        styles {
            element "Software System" {
                background #1168bd
                color #ffffff
            }
            element "Container" {
                background #438dd5
                color #ffffff
            }
            element "Component" {
                background #85bbf0
                color #000000
            }
            element "Person" {
                shape person
                background #08427b
                color #ffffff
            }
        }

        theme default
    }
}
EOF

echo "✅ Generated: $OUTPUT_FILE"
echo
echo "📊 Summary:"
echo "  - Components: ${#components[@]}"
echo "  - Relationships: ${#relationships[@]}"
echo "  - Responsibilities: ${#responsibilities[@]}"
echo
echo "🚀 To view the diagram:"
echo "  docker run -it --rm -p 8080:8080 -v $(dirname $OUTPUT_FILE):/usr/local/structurizr structurizr/lite"
echo "  Then open http://localhost:8080"
