import { $ } from "@wdio/globals";
import assert from "node:assert/strict";

describe("Companion desktop smoke", () => {
  it("abre janela e executa fluxo start/stop", async () => {
    const title = await browser.getTitle();
    assert.equal(title, "Companion Platform");

    const header = await $("h1");
    const headerText = await header.getText();
    assert.match(headerText, /Companion Platform - Prototipo Manual/i);

    const startButton = await $("button=Iniciar sessao");
    await startButton.click();

    const sessionLine = await $("p*=Sessao ativa:");
    await browser.waitUntil(
      async () => {
        const text = await sessionLine.getText();
        return !text.includes("sem sessao ativa");
      },
      {
        timeout: 15000,
        timeoutMsg: "sessao nao ficou ativa apos clicar em Iniciar sessao"
      }
    );

    const stopButton = await $("button=Parar sessao");
    await stopButton.click();

    await browser.waitUntil(
      async () => {
        const text = await sessionLine.getText();
        return text.includes("sem sessao ativa");
      },
      {
        timeout: 15000,
        timeoutMsg: "sessao nao voltou para estado inativo apos clicar em Parar sessao"
      }
    );
  });
});
