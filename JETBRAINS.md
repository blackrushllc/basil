> Q. I would like to create a set of Jetbrains plugins for Yore (PhpStorm) and Basil (RustRover), so that the PhpStorm plugin can make it more useful for Php programmers who are working on a Yore application, such as generating views, boiler-plate classes, and understanding the proprietary "Fred" template engine; and that the RustRover plugin has syntax highlighting and context-sensitive help for the Basil programming language and Basil files.

Love this idea. You’ll get a ton of leverage from IDE support—both for Yore and for Basil. Here’s the quick, no-nonsense map to JetBrains plugin development and where to start.

# How JetBrains plugins are made (in practice)

* **Platform & language.** All IDEs (PhpStorm, RustRover, etc.) are built on the **IntelliJ Platform**. Plugins are written in **Kotlin or Java** against the IntelliJ Platform SDK. You normally build with **Gradle**. ([JetBrains Marketplace][1])
* **Build tool.** Use the **IntelliJ Platform Gradle Plugin (2.x)** to run a sandbox IDE, package, verify, and publish. (This supersedes the legacy “Gradle IntelliJ Plugin 1.x”.) ([JetBrains Marketplace][2])
* **Project scaffold.** Start from the official **IntelliJ Platform Plugin Template** (GitHub). It wires Gradle, CI, run configurations, and docs pointers. ([GitHub][3])
* **Docs hub.** The **IntelliJ Platform Plugin SDK** is the canonical documentation (tutorials, reference, samples). The “Developing a Plugin” page is a great launchpad. ([JetBrains Marketplace][4])

# Two common tracks you’ll need

## 1) Framework/productivity plugin for PhpStorm (Yore)

What you’ll touch:

* **Plugin dependencies.** Your PhpStorm plugin will depend on the PHP plugin: add `<depends>com.jetbrains.php</depends>` and platform/module deps in `plugin.xml`, and declare IDE targets/deps in Gradle. ([JetBrains Marketplace][5])
* **PHP Open API & PSI.** PhpStorm exposes a PHP PSI tree (syntax tree) and helpers you can traverse for inspections, navigation, type providers, etc. ([JetBrains Marketplace][5])
* **IDE integrations you’ll likely add early:**

    * **Project/file templates & wizards** (new Yore controller/model/view, boilerplate). ([JetBrains][6])
    * **Actions/intentions/inspections** (e.g., “Create view for this route”, quick-fix missing class). ([JetBrains Marketplace][1])
    * **Reference contributors + navigation** (click-through from route names or your “Fred” tags to PHP handlers). ([JetBrains Marketplace][1])
    * **Template language support** for **Fred**: start with **language injection** (embed PHP/HTML inside your tags) and/or treat Fred as a **Template Data Language** if appropriate. Later, evolve into full custom language support. ([JetBrains Marketplace][7])

## 2) Language plugin for RustRover (Basil)

You have two “ramp” options:

* **Fastest first step:** import a **TextMate** grammar for instant syntax highlighting of `.basil` files. Great for Day 1 highlighting while you build real features. ([JetBrains][8])
* **Full language support:** use **Grammar-Kit** (BNF + JFlex) to generate lexer, parser & PSI; then implement color settings, folding, completion, resolve, rename, find usages, formatter, and **Quick Documentation** for context-sensitive help. The “Custom Language Support Tutorial” walks through a minimal end-to-end example. ([JetBrains Marketplace][9])

# Where to begin (step-by-step)

1. **Set up your dev environment**

* Install **IntelliJ IDEA (Community is fine)** to develop plugins for any JetBrains IDE. You’ll run a sandboxed PhpStorm/RustRover via Gradle tasks. ([JetBrains Marketplace][4])
* Generate a repo from the **Plugin Template** and open it in IntelliJ. Use Kotlin. ([GitHub][3])
* In `build.gradle.kts`, **target the product** you’re testing (PhpStorm or RustRover) via the **IntelliJ Platform Gradle Plugin** configuration; then run the IDE with your plugin loaded. ([JetBrains Marketplace][10])

2. **Ship a Day-1 MVP for each plugin**

* **Yore/PhpStorm MVP (weekend build):**

    * Register Yore file types & icons; add **New File** templates (Controller, Model, View, Route).
    * Add an **Action**: “Generate Yore CRUD skeleton” (reads your conventions, drops boilerplate).
    * If “Fred” is HTML-like with directives, start with **language injection** so PHP/HTML inside Fred gets full assistance. ([JetBrains Marketplace][7])
* **Basil/RustRover MVP:**

    * Add `.basil` file type + **TextMate** grammar for syntax colors.
    * Add **Quick Documentation** provider that reads from your Basil keyword reference to show concise docs on hover. ([JetBrains][8])

3. **Iterate to pro-level**

* **Yore:** route indexing, reference resolve (jump from route name to controller), inspections (missing view), code completion for Fred tags/filters, gutter icons to run Yore CLI. (You’ll use PSI, reference contributors, inspections.) ([JetBrains Marketplace][1])
* **Basil:** move from TextMate to **Grammar-Kit**: define BNF + lexer, generate PSI, add parser tests, implement completion, formatter, rename/resolve, and **Quick Docs** based on your reference manual. ([GitHub][11])

4. **Testing, compatibility & publishing**

* Use the Gradle plugin tasks to run IDE sandboxes and **Plugin Verifier** for compatibility checks across IDE versions; publish to **JetBrains Marketplace** when ready. ([JetBrains Marketplace][2])

# Great example/training links

* **Start here:** “Developing a Plugin” & the SDK welcome page. ([JetBrains Marketplace][4])
* **Gradle tooling (2.x) & targeting products/deps.** ([JetBrains Marketplace][2])
* **Plugin Template (repo + docs page).** ([GitHub][3])
* **Custom Language Support** (reference + step tutorial). ([JetBrains Marketplace][12])
* **Grammar-Kit** (repo & Gradle plugin). ([GitHub][11])
* **TextMate highlighting in JetBrains IDEs.** ([JetBrains][8])
* **PHP Open API (PhpStorm).** ([JetBrains Marketplace][5])
* **Declaring plugin dependencies & product compatibility.** ([JetBrains Marketplace][13])

# A tiny kickoff plan (so we actually ship)

**Day 1**

* Create two repos from the Plugin Template: `yore-phpstorm-plugin`, `basil-rustrover-plugin`.
* Run each sandbox IDE from Gradle and confirm the plugin shows in **Settings → Plugins**.

**Day 2–3**

* Yore: file templates + a “Generate Controller+View” action.
* Basil: file type + TextMate highlighting; Quick Docs backed by a JSON/YAML of keywords.

**Week 2**

* Yore: add route → controller navigation & an inspection with a quick-fix to create a missing view.
* Basil: start Grammar-Kit BNF; generate PSI; color settings + simple completion.

If you want, I can spin up the initial Gradle configs (with product targets and `com.jetbrains.php` dependency for Yore) and a minimal TextMate grammar for Basil’s tokens to drop into your repos.

[1]: https://plugins.jetbrains.com/docs/intellij/welcome.html?utm_source=chatgpt.com "IntelliJ Platform Plugin SDK"
[2]: https://plugins.jetbrains.com/docs/intellij/tools-intellij-platform-gradle-plugin.html?utm_source=chatgpt.com "IntelliJ Platform Gradle Plugin (2.x)"
[3]: https://github.com/JetBrains/intellij-platform-plugin-template?utm_source=chatgpt.com "JetBrains/intellij-platform-plugin-template"
[4]: https://plugins.jetbrains.com/docs/intellij/developing-plugins.html?utm_source=chatgpt.com "Developing a Plugin | IntelliJ Platform Plugin SDK"
[5]: https://plugins.jetbrains.com/docs/intellij/php-open-api.html?utm_source=chatgpt.com "PHP Open API | IntelliJ Platform Plugin SDK"
[6]: https://www.jetbrains.com/help/idea/using-file-and-code-templates.html?utm_source=chatgpt.com "File templates | IntelliJ IDEA Documentation"
[7]: https://plugins.jetbrains.com/docs/intellij/language-injection.html?utm_source=chatgpt.com "Language Injection | IntelliJ Platform Plugin SDK"
[8]: https://www.jetbrains.com/help/idea/textmate-bundles.html?utm_source=chatgpt.com "TextMate Bundles | IntelliJ IDEA"
[9]: https://plugins.jetbrains.com/docs/intellij/custom-language-support-tutorial.html?utm_source=chatgpt.com "Custom Language Support Tutorial | IntelliJ Platform Plugin ..."
[10]: https://plugins.jetbrains.com/docs/intellij/tools-intellij-platform-gradle-plugin-dependencies-extension.html?utm_source=chatgpt.com "Dependencies Extension | IntelliJ Platform Plugin SDK"
[11]: https://github.com/JetBrains/Grammar-Kit?utm_source=chatgpt.com "JetBrains/Grammar-Kit: Grammar files support & parser/PSI ..."
[12]: https://plugins.jetbrains.com/docs/intellij/custom-language-support.html?utm_source=chatgpt.com "Custom Language Support | IntelliJ Platform Plugin SDK"
[13]: https://plugins.jetbrains.com/docs/intellij/plugin-dependencies.html?utm_source=chatgpt.com "Plugin Dependencies | IntelliJ Platform Plugin SDK"


---

# Repos to create

Create two sibling repos (or subfolders) using the IntelliJ Platform Plugin Template as a starting point.

* `yore-phpstorm-plugin/`
* `basil-rustrover-plugin/`

> Tip: On GitHub, click **Use this template** on JetBrains’ `intellij-platform-plugin-template`, then clone locally.

---

## 1) Yore PhpStorm plugin (framework helpers, Fred awareness)

### Directory layout (minimal MVP)

```
yore-phpstorm-plugin/
  build.gradle.kts
  gradle.properties
  settings.gradle.kts
  src/main/kotlin/
    com/blackrush/yore/GenerateCrudAction.kt
  src/main/resources/
    META-INF/plugin.xml
    icons/yore.svg
    fileTemplates/
      Yore Controller.php.ft
      Yore Model.php.ft
      Yore View.php.ft
```

### `settings.gradle.kts`

```kotlin
rootProject.name = "yore-phpstorm-plugin"
```

### `gradle.properties`

```properties
group=com.blackrush.yore
pluginVersion=0.1.0
# Match your JDK 17+; 21 is fine as of 2024.3/2025.1
kotlin.jvmTarget=21
# Use the latest stable 2.x shown in docs/Marketplace (update as needed)
intellijPlatformVersion=2024.3
```

### `build.gradle.kts`

```kotlin
plugins {
    id("java")
    id("org.jetbrains.kotlin.jvm") version "2.0.21"
    id("org.jetbrains.intellij.platform") version "2.9.0"
}

group = providers.gradleProperty("group").get()
version = providers.gradleProperty("pluginVersion").get()

repositories {
    mavenCentral()
    intellijPlatform {
        defaultRepositories()
    }
}

dependencies {
    intellijPlatform {
        // Target PhpStorm via the 2.x plugin’s typed DSL
        create("PS", providers.gradleProperty("intellijPlatformVersion").get())
        // We need the PHP plugin APIs
        bundledPlugins("com.jetbrains.php")
    }
}

tasks {
    patchPluginXml {
        version.set(providers.gradleProperty("pluginVersion"))
        sinceBuild.set("243") // PhpStorm 2024.3 baseline; adjust as needed
        untilBuild.set(null as String?)
    }

    // Run a PhpStorm sandbox with the plugin loaded
    runIde {
        autoReloadPlugins.set(true)
    }
}
```

### `src/main/resources/META-INF/plugin.xml`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<idea-plugin>
  <id>com.blackrush.yore</id>
  <name>Yore Tools</name>
  <vendor email="support@blackrush.io" url="https://github.com/blackrush">Blackrush LLC</vendor>
  <description>Helpers for developing Yore apps in PhpStorm: CRUD generator, file templates, and (soon) Fred template awareness.</description>

  <depends>com.intellij.modules.platform</depends>
  <!-- Access PhpStorm PHP Open API -->
  <depends>com.jetbrains.php</depends>

  <actions>
    <action id="Yore.GenerateCrudAction"
            class="com.blackrush.yore.GenerateCrudAction"
            text="Yore: Generate CRUD"
            description="Create Controller/Model/View boilerplate for a Yore resource">
      <add-to-group group-id="NewGroup" anchor="last"/>
      <keyboard-shortcut keymap="$default" first-keystroke="ctrl alt G"/>
    </action>
  </actions>
</idea-plugin>
```

### `src/main/kotlin/com/blackrush/yore/GenerateCrudAction.kt`

```kotlin
package com.blackrush.yore

import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.command.WriteCommandAction
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.Messages
import com.intellij.openapi.vfs.VfsUtil
import com.intellij.psi.PsiManager
import java.nio.charset.StandardCharsets

class GenerateCrudAction : AnAction() {
    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return
        val name = Messages.showInputDialog(project, "Resource name (e.g., Customer)", "Yore CRUD", null) ?: return
        WriteCommandAction.runWriteCommandAction(project) {
            generateFiles(project, name)
        }
    }

    private fun generateFiles(project: Project, name: String) {
        val base = project.baseDir ?: return
        val controllerPath = base.findOrCreateChildData(this, "${name}Controller.php")
        val modelPath = base.findOrCreateChildData(this, "${name}.php")
        val viewPath = VfsUtil.createDirectories(base.path + "/views/${name.lowercase()}")
            .findOrCreateChildData(this, "index.php")

        val controllerTpl = loadTemplate(project, "fileTemplates/Yore Controller.php.ft")
            .replace("__NAME__", name)
        val modelTpl = loadTemplate(project, "fileTemplates/Yore Model.php.ft").replace("__NAME__", name)
        val viewTpl = loadTemplate(project, "fileTemplates/Yore View.php.ft").replace("__NAME__", name)

        controllerPath.setBinaryContent(controllerTpl.toByteArray(StandardCharsets.UTF_8))
        modelPath.setBinaryContent(modelTpl.toByteArray(StandardCharsets.UTF_8))
        viewPath.setBinaryContent(viewTpl.toByteArray(StandardCharsets.UTF_8))
    }

    private fun loadTemplate(project: Project, path: String): String {
        val url = this::class.java.classLoader.getResource(path)
            ?: error("Template not found: $path")
        return url.readText()
    }
}
```

### File templates (very small examples)

`src/main/resources/fileTemplates/Yore Controller.php.ft`

```php
<?php
namespace App\Controllers;

use App\Models\__NAME__;

class __NAME__Controller {
    public function index() {
        // TODO: load records and render view
    }
}
```

`src/main/resources/fileTemplates/Yore Model.php.ft`

```php
<?php
namespace App\Models;

class __NAME__ {
    // TODO: properties and ORM mapping
}
```

`src/main/resources/fileTemplates/Yore View.php.ft`

```php
<!-- Fred/HTML view for __NAME__ -->
<div class="container">
  <h1>__NAME__ index</h1>
</div>
```

> Next steps for Yore: add PSI-based reference contributors to navigate from route strings to controllers; inspections that offer a quick-fix to create a missing view; and a Template Data Language or injection strategy for Fred.

---

## 2) Basil RustRover plugin (file type + quick docs; TextMate later)

### Directory layout (minimal MVP)

```
basil-rustrover-plugin/
  build.gradle.kts
  gradle.properties
  settings.gradle.kts
  src/main/kotlin/
    com/blackrush/basil/BasilLanguage.kt
    com/blackrush/basil/BasilFileType.kt
    com/blackrush/basil/BasilQuickDoc.kt
  src/main/resources/
    META-INF/plugin.xml
    basil/keywords.json
    icons/basil.svg
```

### `settings.gradle.kts`

```kotlin
rootProject.name = "basil-rustrover-plugin"
```

### `gradle.properties`

```properties
group=com.blackrush.basil
pluginVersion=0.1.0
kotlin.jvmTarget=21
intellijPlatformVersion=2024.3
```

### `build.gradle.kts`

```kotlin
plugins {
    id("java")
    id("org.jetbrains.kotlin.jvm") version "2.0.21"
    id("org.jetbrains.intellij.platform") version "2.9.0"
}

group = providers.gradleProperty("group").get()
version = providers.gradleProperty("pluginVersion").get()

repositories {
    mavenCentral()
    intellijPlatform { defaultRepositories() }
}

dependencies {
    intellijPlatform {
        // Target RustRover specifically
        create("RR", providers.gradleProperty("intellijPlatformVersion").get())
    }
}

tasks {
    patchPluginXml {
        version.set(providers.gradleProperty("pluginVersion"))
        sinceBuild.set("243")
    }

    runIde { autoReloadPlugins.set(true) }
}
```

### `src/main/resources/META-INF/plugin.xml`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<idea-plugin>
  <id>com.blackrush.basil</id>
  <name>Basil Language Support (MVP)</name>
  <vendor email="support@blackrush.io" url="https://github.com/blackrush">Blackrush LLC</vendor>
  <description>Recognizes .basil files, sets icon and basic Quick Documentation for keywords.</description>

  <depends>com.intellij.modules.platform</depends>

  <extensions defaultExtensionNs="com.intellij">
    <fileType name="Basil"
              language="Basil"
              extensions="basil"
              filetype="com.blackrush.basil.BasilFileType"/>
  </extensions>

  <extensions defaultExtensionNs="com.intellij.lang">
    <language implementationClass="com.blackrush.basil.BasilLanguage"/>
  </extensions>

  <!-- Register Quick Documentation provider -->
  <extensions defaultExtensionNs="com.intellij">
    <editorDocumentationProvider implementation="com.blackrush.basil.BasilQuickDoc"/>
  </extensions>
</idea-plugin>
```

### `src/main/kotlin/com/blackrush/basil/BasilLanguage.kt`

```kotlin
package com.blackrush.basil

import com.intellij.lang.Language

class BasilLanguage private constructor() : Language("Basil") {
    companion object { @JvmField val INSTANCE = BasilLanguage() }
}
```

### `src/main/kotlin/com/blackrush/basil/BasilFileType.kt`

```kotlin
package com.blackrush.basil

import com.intellij.openapi.fileTypes.LanguageFileType
import javax.swing.Icon

object BasilFileType : LanguageFileType(BasilLanguage.INSTANCE) {
    override fun getName() = "Basil"
    override fun getDescription() = "Basil source file"
    override fun getDefaultExtension() = "basil"
    override fun getIcon(): Icon? = null // TODO: return basil.svg via IconLoader
}
```

### `src/main/kotlin/com/blackrush/basil/BasilQuickDoc.kt`

```kotlin
package com.blackrush.basil

import com.intellij.lang.documentation.AbstractDocumentationProvider
import com.intellij.openapi.util.TextRange
import com.intellij.psi.PsiElement
import java.nio.charset.StandardCharsets

class BasilQuickDoc : AbstractDocumentationProvider() {
    private val docs: Map<String, String> by lazy { loadDocs() }

    override fun generateDoc(element: PsiElement?, originalElement: PsiElement?): String? {
        val text = originalElement?.text ?: return null
        val key = text.trim().uppercase()
        return docs[key]?.let { "<b>$key</b><br/>$it" }
    }

    private fun loadDocs(): Map<String, String> {
        val url = this::class.java.classLoader.getResource("basil/keywords.json") ?: return emptyMap()
        val json = url.readText(StandardCharsets.UTF_8)
        return parseJson(json)
    }

    private fun parseJson(json: String): Map<String, String> {
        // Tiny JSON parser to avoid extra deps; keep it simple: {"PRINT":"Writes text..."}
        return json.trim().removePrefix("{").removeSuffix("}")
            .split(Regex(",\n?"))
            .mapNotNull { entry ->
                val parts = entry.split(":", limit = 2)
                if (parts.size == 2) {
                    val k = parts[0].trim().trim('"')
                    val v = parts[1].trim().trim('"')
                    k to v
                } else null
            }.toMap()
    }
}
```

### `src/main/resources/basil/keywords.json`

```json
{
  "PRINT": "Output a value to stdout without a trailing newline unless ';' is used.",
  "INPUT": "Prompt and read a value from stdin.",
  "LET": "Assign a value to a variable.",
  "FOR": "Counted loop: FOR i = start TO end [STEP step] ... NEXT [i]",
  "NEXT": "Advance FOR loop; optional control variable.",
  "IF": "Conditional execution: IF cond THEN ... [ELSE ...]",
  "ELSE": "Alternate branch for IF",
  "FUNC": "Declare function",
  "RETURN": "Return from function",
  "DIM": "Declare array or object reference",
  "BEGIN": "Begin block",
  "END": "End block"
}
```

> Next steps for Basil: add TextMate highlighting for `.basil` (quick win) and then graduate to a Grammar‑Kit parser for full PSI-based features (completion, rename, resolve, formatter).

---

## Running each plugin in a sandbox IDE

From the repo root:

```bash
# Yore / PhpStorm
./gradlew :runIde    # launches PhpStorm 2024.3 sandbox with the plugin

# Basil / RustRover
./gradlew :runIde    # launches RustRover 2024.3 sandbox with the plugin
```

If the wrong IDE launches, ensure the `dependencies { intellijPlatform { create("PS", version) } }` (for Yore) or `create("RR", version)` (for Basil) is present in the respective `build.gradle.kts`.

---

## Optional: TextMate grammar for Basil (drop-in MVP)

1. Author `basil.tmlanguage.json` (even a small one with keywords/strings/comments).
2. In RustRover: **Settings ▶ Editor ▶ TextMate Bundles ▶ +** and select the folder containing `basil.tmlanguage.json` and a theme mapping.
3. To bundle inside the plugin later, add the `com.intellij.textmate` plugin as a dependency and register the bundle at startup; use the SDK docs’ TextMate section as reference.

---

## Publishing/compatibility quick notes

* Keep `sinceBuild` at the current major baseline (e.g., `243` for 2024.3). When 2025.1 lands, decide whether to bump.
* Before publishing to Marketplace, run `./gradlew verifyPlugin` and fix any verifier findings.
* Add a proper SVG icon and a `change-notes` block in `patchPluginXml`.

---

## What to implement next (shortlist)

**Yore (PhpStorm)**

* ReferenceContributor: route names → controllers
* Inspection: missing Fred view → quick‑fix to create it
* Template Data Language or injections for Fred blocks
* Gutter action to run `yore` CLI targets

**Basil (RustRover)**

* TextMate bundle (instant colors)
* Grammar‑Kit lexer/parser + PSI
* Basic completion for keywords and functions
* Quick Docs wired to a richer JSON (generated from your Basil reference)


Awesome — I dropped a ready-to-use starter pack into the canvas with:

* Two minimal plugin skeletons (Yore for PhpStorm, Basil for RustRover)
* Gradle (2.x) configs targeting **PS** and **RR** product types
* `plugin.xml` stubs, Kotlin classes, and file templates
* Quick next steps and run instructions

If you want, I can also generate:

* a tiny **TextMate** grammar for `.basil` so you get instant syntax colors, and
* a first **Fred** template-language injection pass for Yore.

Shall I add those now?
