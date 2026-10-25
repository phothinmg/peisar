## Peisar in browaer

Add in `head` of your HTML

```html
<script src="https://cdn.jsdelivr.net/npm/peisar@1.0.12/browser/peisar.min.js"></script>
```

## Example

```html
 <body>
    <article class="markdown-body" id="md"></article>
    <script>
     const peisar = new Peisar("# Hello World", { fragment: true });
     document.getElementById("md").innerHTML = peisar.html;
    </script>
  </body>
```
