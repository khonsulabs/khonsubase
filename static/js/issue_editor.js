document.addEventListener('DOMContentLoaded', function () {
    const viewer = new toastui.Editor.factory({
        el: document.querySelector("#viewer"),
        usageStatistics: false,
        viewer: {{view_only}},
        initialValue: {{ markdown | json_encode() | safe }}
    });
});