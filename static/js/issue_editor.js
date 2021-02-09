function html_sanitizer(prefix) {
    const pattern = new RegExp(prefix + '(\\d+)', 'g');
    return function (html) {
        const root = document.createElement('div');

        if (typeof html === 'string' || html instanceof String) {
            html = html.replace(/<!--[\s\S]*?-->/g, '');
            root.innerHTML = html;
        } else {
            root.appendChild(html);
        }

        sanitize_node(root, false);

        let result = root.innerHTML.replaceAll(pattern, "<a href=\"/issue/$1\">$&</a>");
        root.remove();
        return result;
    };
}

function sanitize_node(node, sanitize_self) {
    if (sanitize_self) {
        if (node.nodeType === Node.TEXT_NODE || !tagWhitelist[node.tagName]) {
            node.remove();
            return;
        }

        let attrs_to_remove = [];
        for (var i = node.attributes.length - 1; i >= 0; i--) {
            let attribute = node.attributes[i];
            if (!attributeWhitelist[attribute.name]) {
                attrs_to_remove.push(attribute.name);
            }
        }
        attrs_to_remove.forEach(function (attr) {
            node.removeAttribute(attr);
        });
    }

    for (i = node.children.length - 1; i >= 0; i--) {
        sanitize_node(node.children.item(i), true);
    }
}

function toArray(arrayLike) {
    var arr;
    try {
        arr = Array.prototype.slice.call(arrayLike);
    } catch (e) {
        arr = [];
        forEachArray(arrayLike, function(value) {
            arr.push(value);
        });
    }

    return arr;
}

function forEachArray(arr, iteratee, context) {
    var index = 0;
    var len = (arr === undefined) ? 0 : arr.length;

    context = context || null;

    for (; index < len; index += 1) {
        if (iteratee.call(context, arr[index], index, arr) === false) {
            break;
        }
    }
}

var tagWhitelist = {
    'A': true,
    'B': true,
    'BODY': true,
    'BR': true,
    'DIV': true,
    'EM': true,
    'HR': true,
    'I': true,
    'IMG': true,
    'P': true,
    'SPAN': true,
    'STRONG': true,
    'UL': true,
    'OL': true,
    'LI': true,
    'TABLE': true,
    'TR': true,
    'THEAD': true,
    'TBODY': true,
    'TD': true,
    'TH': true,
};

var attributeWhitelist = {
    'href': true,
    'src': true,
    'data-nodeid': true,
};

