import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { Input } from "../../ui/Input";
import { SettingContainer } from "../../ui/SettingContainer";
import { useSettings } from "../../../hooks/useSettings";

export const GeminiApiKey: React.FC = () => {
  const { t } = useTranslation();
  const { settings } = useSettings();
  const apiKey = settings?.post_process_api_keys?.gemini_transcription ?? "";
  const [localValue, setLocalValue] = useState(apiKey);

  // Sync with prop changes
  React.useEffect(() => {
    setLocalValue(apiKey);
  }, [apiKey]);

  const handleBlur = async () => {
    if (localValue !== apiKey) {
      await commands.changePostProcessApiKeySetting(
        "gemini_transcription",
        localValue,
      );
    }
  };

  return (
    <SettingContainer
      title={t("settings.general.geminiApiKey.title")}
      description={t("settings.general.geminiApiKey.description")}
      descriptionMode="tooltip"
      grouped={true}
    >
      <Input
        type="password"
        value={localValue}
        onChange={(e) => setLocalValue(e.target.value)}
        onBlur={handleBlur}
        placeholder="AIza..."
        variant="compact"
        className="flex-1 min-w-[280px]"
      />
    </SettingContainer>
  );
};
