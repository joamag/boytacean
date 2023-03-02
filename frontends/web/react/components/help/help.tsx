import { Link } from "emukit";
import React, { FC } from "react";

import "./help.css";

export const HelpKeyboard: FC = () => (
    <ul className="keyboard-help">
        <li>
            <span className="key-container">
                <span className="key">Enter</span>
            </span>
            Start button
        </li>
        <li>
            <span className="key-container">
                <span className="key">Space</span>
            </span>
            Select button
        </li>
        <li>
            <span className="key-container">
                <span className="key">A</span>
            </span>
            A button
        </li>
        <li>
            <span className="key-container">
                <span className="key">S</span>
            </span>
            B button
        </li>
        <li>
            <span className="key-container">
                <span className="key">←</span>
                <span className="key">→</span>
            </span>
            Horizontal control
        </li>
        <li>
            <span className="key-container">
                <span className="key">↑</span>
                <span className="key">↓</span>
            </span>
            Vertical control
        </li>
        <li>
            <span className="key-container">
                <span className="key">Escape</span>
            </span>
            Exit fullscreen
        </li>
        <li>
            <span className="key-container">
                <span className="key">Ctrl + D</span>
            </span>
            Turbo speed
        </li>
        <li>
            <span className="key-container">
                <span className="key">Ctrl + F</span>
            </span>
            Toggle fullscreen
        </li>
        <li>
            <span className="key-container">
                <span className="key">Ctrl + K</span>
            </span>
            Toggle on-screen keyboard
        </li>
        <li>
            <span className="key-container">
                <span className="key">Ctrl + P</span>
            </span>
            Change screen palette
        </li>
    </ul>
);

export const HelpFaqs: FC = () => (
    <div className="faqs-help">
        <h3>Does it play all Game Boy games?</h3>
        <p>
            Not really, but it plays the coolest ones. Now seriously it should
            play around 90% of the Game Boy games.
        </p>
        <h3>Why there's no sound?</h3>
        <p>
            It's under development, I'm hopping to have it before end of 2023.
        </p>
        <h3>Can I use my Xbox One/PS4/PS5 Gamepad?</h3>
        <p>
            Yes, just plug it in, press a button, and you're ready to go.
            <br />
            BTW: This uses the{" "}
            <Link
                href="https://developer.mozilla.org/docs/Web/API/Gamepad_API/Using_the_Gamepad_API"
                target="_blank"
            >
                Web Gamepad API
            </Link>{" "}
            🕹️.
        </p>
        <h3>Will it ever play Game Boy Color games?</h3>
        <p>Eventually...</p>
        <h3>I've found a bug, where can I report it?</h3>
        <p>
            Use the{" "}
            <Link
                href="https://github.com/joamag/boytacean/issues"
                target="_blank"
            >
                GitHub issue tracker
            </Link>
            .
        </p>
        <h3>What's WebAssembly?</h3>
        <p>
            The coolest thing that happened to the Web in the latest years,
            check the{" "}
            <Link
                href="https://en.wikipedia.org/wiki/WebAssembly"
                target="_blank"
            >
                Wikipedia page
            </Link>{" "}
            for more info.
        </p>
        <h3>Why another Game Boy emulator?</h3>
        <p>Because it's cool, and a challenging problem to solve.</p>
    </div>
);
