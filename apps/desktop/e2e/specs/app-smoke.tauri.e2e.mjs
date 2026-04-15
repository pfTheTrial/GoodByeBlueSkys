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

    const startVoiceButton = await $("button=Iniciar voz");
    await startVoiceButton.click();

    const voiceLine = await $("p*=Status voz:");
    await browser.waitUntil(
      async () => {
        const text = await voiceLine.getText();
        return text.includes("ativa");
      },
      {
        timeout: 15000,
        timeoutMsg: "voz nao ficou ativa apos clicar em Iniciar voz"
      }
    );

    const sendVoiceInputButton = await $("button=Enviar chunk input");
    await sendVoiceInputButton.click();

    const lastVoiceEventLine = await $("p*=Ultimo evento voz:");
    await browser.waitUntil(
      async () => {
        const text = await lastVoiceEventLine.getText();
        return text.includes("input:512");
      },
      {
        timeout: 15000,
        timeoutMsg: "evento de input de voz nao foi registrado"
      }
    );

    const publishVoiceOutputButton = await $("button=Publicar chunk output");
    await publishVoiceOutputButton.click();

    await browser.waitUntil(
      async () => {
        const text = await lastVoiceEventLine.getText();
        return text.includes("output:1024");
      },
      {
        timeout: 15000,
        timeoutMsg: "evento de output de voz nao foi registrado"
      }
    );

    const stopVoiceButton = await $("button=Parar voz");
    await stopVoiceButton.click();

    await browser.waitUntil(
      async () => {
        const text = await voiceLine.getText();
        return text.includes("inativa");
      },
      {
        timeout: 15000,
        timeoutMsg: "voz nao voltou para estado inativo apos clicar em Parar voz"
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
