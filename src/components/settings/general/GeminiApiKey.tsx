import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Save, Check } from "lucide-react";
import { commands } from "@/bindings";
import { Input } from "../../ui/Input";
import { SettingContainer } from "../../ui/SettingContainer";
import { useSettings } from "../../../hooks/useSettings";

export const GeminiApiKey: React.FC = () => {
  const { t } = useTranslation();
  const { settings, refreshSettings } = useSettings();
  const apiKey = settings?.post_process_api_keys?.gemini_transcription ?? "";
  const [localValue, setLocalValue] = useState(apiKey);
  const [isSaving, setIsSaving] = useState(false);
  const [justSaved, setJustSaved] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // Sync with prop changes
  useEffect(() => {
    setLocalValue(apiKey);
    setErrorMessage(null);
  }, [apiKey]);

  const hasUnsavedChanges = localValue !== apiKey;

  const handleSave = async () => {
    setIsSaving(true);
    setErrorMessage(null);
    try {
      await commands.changePostProcessApiKeySetting(
        "gemini_transcription",
        localValue,
      );
      // Refresh settings to update the store with the saved value
      await refreshSettings();
      setJustSaved(true);
      setTimeout(() => setJustSaved(false), 2000);
    } catch (error) {
      console.error("Failed to save API key:", error);
      setErrorMessage(
        error instanceof Error ? error.message : "Failed to save API key"
      );
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <SettingContainer
      title={t("settings.general.geminiApiKey.title")}
      description={t("settings.general.geminiApiKey.description")}
      descriptionMode="tooltip"
      grouped={true}
    >
      <div className="flex flex-col gap-2 flex-1 min-w-[280px]">
        <div className="flex items-center gap-2">
          <Input
            type="password"
            value={localValue}
            onChange={(e) => setLocalValue(e.target.value)}
            placeholder="AIza..."
            variant="compact"
            className="flex-1"
          />
          {(hasUnsavedChanges || justSaved) && (
            <button
              onClick={handleSave}
              disabled={isSaving || justSaved}
              className={`px-3 py-1.5 rounded-lg font-medium transition-colors ${
                justSaved
                  ? "bg-green-500 text-white cursor-default"
                  : "bg-blue-500 hover:bg-blue-600 text-white"
              } ${isSaving ? "opacity-50 cursor-wait" : ""}`}
            >
              {justSaved ? (
                <Check size={18} />
              ) : (
                <Save size={18} />
              )}
            </button>
          )}
        </div>
        {errorMessage && (
          <div className="text-red-500 text-sm">{errorMessage}</div>
        )}
      </div>
    </SettingContainer>
  );
};
