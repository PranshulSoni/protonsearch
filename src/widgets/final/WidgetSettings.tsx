import { useId } from 'react';
import './final-widgets.css';
import { FINAL_SETTING_FIELDS, settingsFor } from './meta';
import type { FinalWidgetSettings, WidgetSettingsProps } from './types';

export function WidgetSettings({ id, value, onChange, disabled = false }: WidgetSettingsProps) {
  const baseId = useId();
  const current = settingsFor(id, value) as unknown as Record<string, unknown>;
  const fields = FINAL_SETTING_FIELDS[id];

  function update(key: string, incoming: unknown) {
    onChange(settingsFor(id, { ...current, [key]: incoming }));
  }

  return (
    <fieldset className="fw-settings" disabled={disabled}>
      <legend className="fw-settings-legend">Widget settings</legend>
      <div className="fw-settings-list">
        {fields.map((field) => {
          const inputId = `${baseId}-${field.key}`;
          const descriptionId = `${inputId}-description`;
          const fieldValue = current[field.key];

          if (field.type === 'toggle') {
            return (
              <div className="fw-setting fw-setting--toggle" key={field.key}>
                <div className="fw-setting-copy">
                  <label htmlFor={inputId}>{field.label}</label>
                  <small id={descriptionId}>{field.description}</small>
                </div>
                <label className="fw-switch" htmlFor={inputId}>
                  <input
                    id={inputId}
                    type="checkbox"
                    checked={fieldValue === true}
                    onChange={(event) => update(field.key, event.target.checked)}
                    aria-label={field.label}
                    aria-describedby={descriptionId}
                  />
                  <span className="fw-switch-track" aria-hidden="true">
                    <span className="fw-switch-thumb" />
                  </span>
                </label>
              </div>
            );
          }

          if (field.type === 'range') {
            const outputId = `${inputId}-output`;
            return (
              <div className="fw-setting fw-setting--range" key={field.key}>
                <div className="fw-setting-line">
                  <label htmlFor={inputId}>{field.label}</label>
                  <output htmlFor={inputId} id={outputId} className="fw-setting-value">
                    {String(fieldValue)}
                    {field.suffix}
                  </output>
                </div>
                <input
                  id={inputId}
                  className="fw-range"
                  type="range"
                  min={field.min}
                  max={field.max}
                  step={field.step}
                  value={Number(fieldValue)}
                  onChange={(event) => update(field.key, Number(event.target.value))}
                  aria-describedby={descriptionId}
                  aria-valuetext={`${String(fieldValue)}${field.suffix}`}
                />
                <small id={descriptionId}>{field.description}</small>
              </div>
            );
          }

          if (field.type === 'number') {
            return (
              <div className="fw-setting fw-setting--number" key={field.key}>
                <label htmlFor={inputId}>{field.label}</label>
                <input
                  id={inputId}
                  className="fw-input fw-number-input"
                  type="number"
                  min={field.min}
                  max={field.max}
                  step={field.step}
                  value={Number(fieldValue ?? 0)}
                  placeholder={field.placeholder}
                  onChange={(event) => {
                    const incoming = event.target.valueAsNumber;
                    if (Number.isFinite(incoming)) update(field.key, incoming);
                  }}
                  aria-describedby={descriptionId}
                />
                <small id={descriptionId}>{field.description}</small>
              </div>
            );
          }

          if (field.type === 'select') {
            return (
              <div className="fw-setting fw-setting--select" key={field.key}>
                <label htmlFor={inputId}>{field.label}</label>
                <select
                  id={inputId}
                  className="fw-select"
                  value={String(fieldValue)}
                  onChange={(event) => update(field.key, event.target.value)}
                  aria-describedby={descriptionId}
                >
                  {field.options.map((option) => (
                    <option key={option.value} value={option.value}>{option.label}</option>
                  ))}
                </select>
                <small id={descriptionId}>{field.description}</small>
              </div>
            );
          }

          return (
            <div className="fw-setting fw-setting--text" key={field.key}>
              <label htmlFor={inputId}>{field.label}</label>
              <input
                id={inputId}
                className="fw-input"
                type="text"
                value={String(fieldValue ?? '')}
                placeholder={field.placeholder}
                maxLength={field.maxLength}
                onChange={(event) => update(field.key, event.target.value)}
                aria-describedby={descriptionId}
              />
              <small id={descriptionId}>{field.description}</small>
            </div>
          );
        })}
      </div>
    </fieldset>
  );
}

export function settingsForWidget(id: WidgetSettingsProps['id'], value?: FinalWidgetSettings | null) {
  return settingsFor(id, value);
}

export default WidgetSettings;
