'use strict';

function highlight() {
    var line = location.hash;
    var highlighted = document.getElementsByClassName('highlighted');
    for (var i = 0; i < highlighted.length; ++ i) {
        var e = highlighted[i];
        e.className = e.className.replace(/\bhighlighted\b/g, '');
    }
    if (line) {
        var lineElement = document.getElementById(line.substr(1));
        if (!lineElement) {
            return;
        }
        lineElement.className += ' highlighted';
        if (lineElement.scrollIntoViewIfNeeded) {
            lineElement.scrollIntoViewIfNeeded(true);
        } else if (lineElement.scrollIntoView) {
            lineElement.scrollIntoView(true);
        }
    }
};

window.onhashchange = highlight;
window.onload = highlight;


var arrowIdSeq = 0;

/** @param {MouseEvent} e */
function drawArrow(e) {
    var target = e.target;
    if (!/\bbr-local\b/.test(target.className)) {
        return;
    }
    var line = target.href.match(/\#(\d+)$/)[1];
    var tr = document.getElementById(line);
    if (!tr) {
        return;
    }

    var branchCount = target.getAttribute('data-branch-count') | 0;

    var td = tr.getElementsByTagName('td')[2];
    var rect = td.getBoundingClientRect();
    var arrowEndX = rect.left + rect.width/3;
    var arrowEndY = rect.top + rect.height/2;

    rect = target.getBoundingClientRect();
    var arrowStartX = rect.left + rect.width/2;
    var arrowStartY = rect.top + rect.height/2;

    var canvasWidth = Math.abs(arrowEndX - arrowStartX);
    var canvasHeight = Math.max(Math.abs(arrowEndY - arrowStartY), 2);
    var scrollTop = document.scrollingElement.scrollTop;

    var canvas = document.createElement('div');
    canvas.id = 'arrow-' + arrowIdSeq;
    arrowIdSeq ++;
    canvas.className = (arrowEndY >= arrowStartY) ? 'arrow-down' : 'arrow-up';
    if (!branchCount) {
        canvas.className += ' arrow-zero';
    }
    canvas.style.width = canvasWidth + 'px';
    canvas.style.height = canvasHeight + 'px';
    canvas.style.left = arrowStartX + 'px';
    canvas.style.top = (Math.min(arrowStartY, arrowEndY) + scrollTop) + 'px';
    canvas.style.position = 'absolute';
    target.associatedCanvas = canvas;

    var arrowHead = document.createElement('div');
    arrowHead.className = 'arrow-head';
    canvas.appendChild(arrowHead);

    var note = document.createElement('div');
    note.className = 'arrow-note';
    note.innerHTML = branchCount ? 'taken ×' + branchCount : 'not taken';
    canvas.appendChild(note);

    document.body.appendChild(canvas);
}

/** @param {MouseEvent} e */
function clearArrow(e) {
    var target = e.target;
    if (!/\bbr-local\b/.test(target.className)) {
        return;
    }
    var canvas = target.associatedCanvas;
    if (canvas) {
        target.associatedCanvas = undefined;
        document.body.removeChild(canvas);
    }
}

/** @param e {MouseEvent} */
function followArrow(e) {
    var target = e.target;
    if (target.tagName !== 'A') {
        return;
    }
    e.preventDefault();
    e.stopPropagation();
    history.replaceState(null, null, target.href);
    highlight();
}

var sourceElement = document.getElementById('source');
sourceElement.onmouseover = drawArrow;
sourceElement.onmouseout = clearArrow;
if (history.replaceState) {
    sourceElement.onclick = followArrow;
}
