import {
  addCompatibilityRule,
  addConnectorClass,
  updateCompatibilityRule,
  updateConnectorClass,
} from "../editorState";
import type { EditorPackState } from "../types";

interface RulesEditorProps {
  state: EditorPackState;
  onStateChange: (state: EditorPackState) => void;
}

export default function RulesEditor({ state, onStateChange }: RulesEditorProps) {
  return (
    <section className="rules-editor">
      <div className="section-heading">
        <h2>Connector Classes</h2>
        <button
          type="button"
          onClick={() => {
            const className = nextClassName(state);
            onStateChange(
              addConnectorClass(state, className, titleFromClassName(className)),
            );
          }}
        >
          Add Class
        </button>
      </div>
      {state.pack.connector_classes.map((connectorClass, index) => (
        <div className="rule-row class-row" key={`class-${index}`}>
          <input
            aria-label={`Class ${index + 1} id`}
            value={connectorClass.class}
            onChange={(event) =>
              onStateChange(
                updateConnectorClass(state, index, {
                  ...connectorClass,
                  class: event.currentTarget.value,
                }),
              )
            }
          />
          <input
            aria-label={`Class ${index + 1} name`}
            value={connectorClass.display_name}
            onChange={(event) =>
              onStateChange(
                updateConnectorClass(state, index, {
                  ...connectorClass,
                  display_name: event.currentTarget.value,
                }),
              )
            }
          />
        </div>
      ))}

      <div className="section-heading">
        <h2>Compatibility Rules</h2>
        <button
          type="button"
          disabled={state.pack.connector_classes.length === 0}
          onClick={() => {
            const className = state.pack.connector_classes[0].class;
            onStateChange(addCompatibilityRule(state, className, className));
          }}
        >
          Add Rule
        </button>
      </div>
      {state.pack.compatibility_rules.map((rule, index) => (
        <div className="rule-row" key={`rule-${index}`}>
          <select
            aria-label={`Rule ${index + 1} first class`}
            value={rule.a_class}
            onChange={(event) =>
              onStateChange(
                updateCompatibilityRule(state, index, {
                  ...rule,
                  a_class: event.currentTarget.value,
                }),
              )
            }
          >
            {state.pack.connector_classes.map((connectorClass, classIndex) => (
              <option
                key={`${connectorClass.class}-${classIndex}`}
                value={connectorClass.class}
              >
                {connectorClass.display_name}
              </option>
            ))}
          </select>
          <select
            aria-label={`Rule ${index + 1} second class`}
            value={rule.b_class}
            onChange={(event) =>
              onStateChange(
                updateCompatibilityRule(state, index, {
                  ...rule,
                  b_class: event.currentTarget.value,
                }),
              )
            }
          >
            {state.pack.connector_classes.map((connectorClass, classIndex) => (
              <option
                key={`${connectorClass.class}-${classIndex}`}
                value={connectorClass.class}
              >
                {connectorClass.display_name}
              </option>
            ))}
          </select>
          <select
            aria-label={`Rule ${index + 1} rotation`}
            value={rule.rotation.kind}
            onChange={(event) =>
              onStateChange(
                updateCompatibilityRule(state, index, {
                  ...rule,
                  rotation:
                    event.currentTarget.value === "free"
                      ? { kind: "free" }
                      : { kind: "locked" },
                }),
              )
            }
          >
            <option value="locked">Locked</option>
            <option value="free">Free</option>
          </select>
        </div>
      ))}
    </section>
  );
}

function nextClassName(state: EditorPackState): string {
  const existing = new Set(
    state.pack.connector_classes.map((connectorClass) => connectorClass.class),
  );
  let index = 1;

  while (existing.has(`class_${index}`)) {
    index += 1;
  }

  return `class_${index}`;
}

function titleFromClassName(className: string): string {
  return className
    .split("_")
    .filter((part) => part.length > 0)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}
