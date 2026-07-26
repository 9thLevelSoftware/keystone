import type { ControlledVocabulary, EditorPackState, PackProvenance } from "../types";

interface PackSettingsProps {
  state: EditorPackState;
  onStateChange: (state: EditorPackState) => void;
}

export default function PackSettings({ state, onStateChange }: PackSettingsProps) {
  const pack = state.pack;

  function patchPack(
    patch: Partial<typeof pack> & {
      provenance?: PackProvenance;
      vocabulary?: ControlledVocabulary;
    },
  ) {
    onStateChange({
      ...state,
      dirty: true,
      pack: {
        ...pack,
        ...patch,
      },
    });
  }

  function patchProvenance(patch: Partial<PackProvenance>) {
    patchPack({
      provenance: {
        ...pack.provenance,
        ...patch,
      },
    });
  }

  function patchVocabulary(patch: Partial<ControlledVocabulary>) {
    patchPack({
      vocabulary: {
        ...pack.vocabulary,
        ...patch,
      },
    });
  }

  function listEditor(
    label: string,
    key: keyof Pick<
      ControlledVocabulary,
      "semantic_tags" | "affordances" | "placement_constraints"
    >,
  ) {
    const value = pack.vocabulary[key].join(", ");
    return (
      <label>
        {label} (comma-separated)
        <textarea
          rows={2}
          value={value}
          onChange={(event) => {
            const terms = event.currentTarget.value
              .split(",")
              .map((part) => part.trim())
              .filter((part) => part.length > 0);
            patchVocabulary({ [key]: terms });
          }}
        />
      </label>
    );
  }

  return (
    <section className="pack-settings" aria-label="Pack settings">
      <h2>Pack</h2>
      <label>
        Display name
        <input
          value={pack.display_name}
          onChange={(event) => patchPack({ display_name: event.currentTarget.value })}
        />
      </label>
      <label>
        License summary (required)
        <input
          value={pack.license_summary}
          onChange={(event) => patchPack({ license_summary: event.currentTarget.value })}
          placeholder="e.g. CC0, MIT, proprietary — all rights reserved"
        />
      </label>

      <h3>Provenance</h3>
      <label>
        Source
        <input
          value={pack.provenance.source ?? ""}
          onChange={(event) =>
            patchProvenance({ source: event.currentTarget.value || null })
          }
        />
      </label>
      <label>
        Author
        <input
          value={pack.provenance.author ?? ""}
          onChange={(event) =>
            patchProvenance({ author: event.currentTarget.value || null })
          }
        />
      </label>
      <label>
        Created at
        <input
          value={pack.provenance.created_at ?? ""}
          onChange={(event) =>
            patchProvenance({ created_at: event.currentTarget.value || null })
          }
          placeholder="ISO-8601 date"
        />
      </label>
      <label>
        Notes
        <textarea
          rows={2}
          value={pack.provenance.notes ?? ""}
          onChange={(event) =>
            patchProvenance({ notes: event.currentTarget.value || null })
          }
        />
      </label>

      <h3>Controlled vocabulary</h3>
      <label className="checkbox-row">
        <input
          type="checkbox"
          checked={pack.vocabulary.allow_namespaced_extensions}
          onChange={(event) =>
            patchVocabulary({
              allow_namespaced_extensions: event.currentTarget.checked,
            })
          }
        />
        Allow namespaced extensions (project:tag)
      </label>
      {listEditor("Semantic tags", "semantic_tags")}
      {listEditor("Affordances", "affordances")}
      {listEditor("Placement constraints", "placement_constraints")}
    </section>
  );
}
