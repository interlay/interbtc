"use strict";(function(){const itemTypes=["mod","externcrate","import","struct","enum","fn","type","static","trait","impl","tymethod","method","structfield","variant","macro","primitive","associatedtype","constant","associatedconstant","union","foreigntype","keyword","existential","attr","derive","traitalias",];const TY_PRIMITIVE=itemTypes.indexOf("primitive");const TY_KEYWORD=itemTypes.indexOf("keyword");const ROOT_PATH=typeof window!=="undefined"?window.rootPath:"../";function hasOwnPropertyRustdoc(obj,property){return Object.prototype.hasOwnProperty.call(obj,property)}function printTab(nb){let iter=0;let foundCurrentTab=false;let foundCurrentResultSet=false;onEachLazy(document.getElementById("search-tabs").childNodes,elem=>{if(nb===iter){addClass(elem,"selected");foundCurrentTab=true}else{removeClass(elem,"selected")}iter+=1});iter=0;onEachLazy(document.getElementById("results").childNodes,elem=>{if(nb===iter){addClass(elem,"active");foundCurrentResultSet=true}else{removeClass(elem,"active")}iter+=1});if(foundCurrentTab&&foundCurrentResultSet){searchState.currentTab=nb}else if(nb!==0){printTab(0)}}const editDistanceState={current:[],prev:[],prevPrev:[],calculate:function calculate(a,b,limit){if(a.length<b.length){const aTmp=a;a=b;b=aTmp}const minDist=a.length-b.length;if(minDist>limit){return limit+1}while(b.length>0&&b[0]===a[0]){a=a.substring(1);b=b.substring(1)}while(b.length>0&&b[b.length-1]===a[a.length-1]){a=a.substring(0,a.length-1);b=b.substring(0,b.length-1)}if(b.length===0){return minDist}const aLength=a.length;const bLength=b.length;for(let i=0;i<=bLength;++i){this.current[i]=0;this.prev[i]=i;this.prevPrev[i]=Number.MAX_VALUE}for(let i=1;i<=aLength;++i){this.current[0]=i;const aIdx=i-1;for(let j=1;j<=bLength;++j){const bIdx=j-1;const substitutionCost=a[aIdx]===b[bIdx]?0:1;this.current[j]=Math.min(this.prev[j]+1,this.current[j-1]+1,this.prev[j-1]+substitutionCost);if((i>1)&&(j>1)&&(a[aIdx]===b[bIdx-1])&&(a[aIdx-1]===b[bIdx])){this.current[j]=Math.min(this.current[j],this.prevPrev[j-2]+1)}}const prevPrevTmp=this.prevPrev;this.prevPrev=this.prev;this.prev=this.current;this.current=prevPrevTmp}const distance=this.prev[bLength];return distance<=limit?distance:(limit+1)},};function editDistance(a,b,limit){return editDistanceState.calculate(a,b,limit)}function initSearch(rawSearchIndex){const MAX_RESULTS=200;const NO_TYPE_FILTER=-1;let searchIndex;let currentResults;const ALIASES=Object.create(null);function isWhitespace(c){return" \t\n\r".indexOf(c)!==-1}function isSpecialStartCharacter(c){return"<\"".indexOf(c)!==-1}function isEndCharacter(c){return",>-".indexOf(c)!==-1}function isStopCharacter(c){return isWhitespace(c)||isEndCharacter(c)}function isErrorCharacter(c){return"()".indexOf(c)!==-1}function itemTypeFromName(typename){const index=itemTypes.findIndex(i=>i===typename);if(index<0){throw["Unknown type filter ",typename]}return index}function getStringElem(query,parserState,isInGenerics){if(isInGenerics){throw["Unexpected ","\""," in generics"]}else if(query.literalSearch){throw["Cannot have more than one literal search element"]}else if(parserState.totalElems-parserState.genericsElems>0){throw["Cannot use literal search when there is more than one element"]}parserState.pos+=1;const start=parserState.pos;const end=getIdentEndPosition(parserState);if(parserState.pos>=parserState.length){throw["Unclosed ","\""]}else if(parserState.userQuery[end]!=="\""){throw["Unexpected ",parserState.userQuery[end]," in a string element"]}else if(start===end){throw["Cannot have empty string element"]}parserState.pos+=1;query.literalSearch=true}function isPathStart(parserState){return parserState.userQuery.slice(parserState.pos,parserState.pos+2)==="::"}function isReturnArrow(parserState){return parserState.userQuery.slice(parserState.pos,parserState.pos+2)==="->"}function isIdentCharacter(c){return(c==="_"||(c>="0"&&c<="9")||(c>="a"&&c<="z")||(c>="A"&&c<="Z"))}function isSeparatorCharacter(c){return c===","||isWhitespaceCharacter(c)}function isWhitespaceCharacter(c){return c===" "||c==="\t"}function createQueryElement(query,parserState,name,generics,isInGenerics){if(name==="*"||(name.length===0&&generics.length===0)){return}if(query.literalSearch&&parserState.totalElems-parserState.genericsElems>0){throw["You cannot have more than one element if you use quotes"]}const pathSegments=name.split("::");if(pathSegments.length>1){for(let i=0,len=pathSegments.length;i<len;++i){const pathSegment=pathSegments[i];if(pathSegment.length===0){if(i===0){throw["Paths cannot start with ","::"]}else if(i+1===len){throw["Paths cannot end with ","::"]}throw["Unexpected ","::::"]}}}if(pathSegments.length===0||(pathSegments.length===1&&pathSegments[0]==="")){throw["Found generics without a path"]}parserState.totalElems+=1;if(isInGenerics){parserState.genericsElems+=1}const typeFilter=parserState.typeFilter;parserState.typeFilter=null;return{name:name,fullPath:pathSegments,pathWithoutLast:pathSegments.slice(0,pathSegments.length-1),pathLast:pathSegments[pathSegments.length-1],generics:generics,typeFilter,}}function getIdentEndPosition(parserState){const start=parserState.pos;let end=parserState.pos;let foundExclamation=-1;while(parserState.pos<parserState.length){const c=parserState.userQuery[parserState.pos];if(!isIdentCharacter(c)){if(c==="!"){if(foundExclamation!==-1){throw["Cannot have more than one ","!"," in an ident"]}else if(parserState.pos+1<parserState.length&&isIdentCharacter(parserState.userQuery[parserState.pos+1])){throw["Unexpected ","!",": it can only be at the end of an ident"]}foundExclamation=parserState.pos}else if(isErrorCharacter(c)){throw["Unexpected ",c]}else if(isStopCharacter(c)||isSpecialStartCharacter(c)||isSeparatorCharacter(c)){break}else if(c===":"){if(!isPathStart(parserState)){break}if(foundExclamation!==-1){if(start<=(end-2)){throw["Cannot have associated items in macros"]}else{foundExclamation=-1}}parserState.pos+=1}else{throw["Unexpected ",c]}}parserState.pos+=1;end=parserState.pos}if(foundExclamation!==-1&&start<=(end-2)){if(parserState.typeFilter===null){parserState.typeFilter="macro"}else if(parserState.typeFilter!=="macro"){throw["Invalid search type: macro ","!"," and ",parserState.typeFilter," both specified",]}end=foundExclamation}return end}function getNextElem(query,parserState,elems,isInGenerics){const generics=[];let start=parserState.pos;let end;if(parserState.userQuery[parserState.pos]==="\""){start+=1;getStringElem(query,parserState,isInGenerics);end=parserState.pos-1}else{end=getIdentEndPosition(parserState)}if(parserState.pos<parserState.length&&parserState.userQuery[parserState.pos]==="<"){if(isInGenerics){throw["Unexpected ","<"," after ","<"]}else if(start>=end){throw["Found generics without a path"]}parserState.pos+=1;getItemsBefore(query,parserState,generics,">")}if(start>=end&&generics.length===0){return}elems.push(createQueryElement(query,parserState,parserState.userQuery.slice(start,end),generics,isInGenerics))}function getItemsBefore(query,parserState,elems,endChar){let foundStopChar=true;let start=parserState.pos;const oldTypeFilter=parserState.typeFilter;parserState.typeFilter=null;while(parserState.pos<parserState.length){const c=parserState.userQuery[parserState.pos];if(c===endChar){break}else if(isSeparatorCharacter(c)){parserState.pos+=1;foundStopChar=true;continue}else if(c===":"&&isPathStart(parserState)){throw["Unexpected ","::",": paths cannot start with ","::"]}else if(c===":"){if(parserState.typeFilter!==null){throw["Unexpected ",":"]}if(elems.length===0){throw["Expected type filter before ",":"]}else if(query.literalSearch){throw["You cannot use quotes on type filter"]}const typeFilterElem=elems.pop();checkExtraTypeFilterCharacters(start,parserState);parserState.typeFilter=typeFilterElem.name;parserState.pos+=1;parserState.totalElems-=1;query.literalSearch=false;foundStopChar=true;continue}else if(isEndCharacter(c)){let extra="";if(endChar===">"){extra="<"}else if(endChar===""){extra="->"}else{extra=endChar}throw["Unexpected ",c," after ",extra]}if(!foundStopChar){if(endChar!==""){throw["Expected ",",",", ","&nbsp;"," or ",endChar,", found ",c,]}throw["Expected ",","," or ","&nbsp;",", found ",c,]}const posBefore=parserState.pos;start=parserState.pos;getNextElem(query,parserState,elems,endChar===">");if(endChar!==""&&parserState.pos>=parserState.length){throw["Unclosed ","<"]}if(posBefore===parserState.pos){parserState.pos+=1}foundStopChar=false}if(parserState.pos>=parserState.length&&endChar!==""){throw["Unclosed ","<"]}parserState.pos+=1;parserState.typeFilter=oldTypeFilter}function checkExtraTypeFilterCharacters(start,parserState){const query=parserState.userQuery;for(let pos=start;pos<parserState.pos;++pos){if(!isIdentCharacter(query[pos])&&!isWhitespaceCharacter(query[pos])){throw["Unexpected ",query[pos]," in type filter"]}}}function parseInput(query,parserState){let foundStopChar=true;let start=parserState.pos;while(parserState.pos<parserState.length){const c=parserState.userQuery[parserState.pos];if(isStopCharacter(c)){foundStopChar=true;if(isSeparatorCharacter(c)){parserState.pos+=1;continue}else if(c==="-"||c===">"){if(isReturnArrow(parserState)){break}throw["Unexpected ",c," (did you mean ","->","?)"]}throw["Unexpected ",c]}else if(c===":"&&!isPathStart(parserState)){if(parserState.typeFilter!==null){throw["Unexpected ",":"]}if(query.elems.length===0){throw["Expected type filter before ",":"]}else if(query.literalSearch){throw["You cannot use quotes on type filter"]}const typeFilterElem=query.elems.pop();checkExtraTypeFilterCharacters(start,parserState);parserState.typeFilter=typeFilterElem.name;parserState.pos+=1;parserState.totalElems-=1;query.literalSearch=false;foundStopChar=true;continue}if(!foundStopChar){if(parserState.typeFilter!==null){throw["Expected ",",",", ","&nbsp;"," or ","->",", found ",c,]}throw["Expected ",",",", ","&nbsp;",", ",":"," or ","->",", found ",c,]}const before=query.elems.length;start=parserState.pos;getNextElem(query,parserState,query.elems,false);if(query.elems.length===before){parserState.pos+=1}foundStopChar=false}if(parserState.typeFilter!==null){throw["Unexpected ",":"," (expected path after type filter)"]}while(parserState.pos<parserState.length){if(isReturnArrow(parserState)){parserState.pos+=2;getItemsBefore(query,parserState,query.returned,"");if(query.returned.length===0){throw["Expected at least one item after ","->"]}break}else{parserState.pos+=1}}}function newParsedQuery(userQuery){return{original:userQuery,userQuery:userQuery.toLowerCase(),elems:[],returned:[],foundElems:0,literalSearch:false,error:null,}}function buildUrl(search,filterCrates){let extra="?search="+encodeURIComponent(search);if(filterCrates!==null){extra+="&filter-crate="+encodeURIComponent(filterCrates)}return getNakedUrl()+extra+window.location.hash}function getFilterCrates(){const elem=document.getElementById("crate-search");if(elem&&elem.value!=="all crates"&&hasOwnPropertyRustdoc(rawSearchIndex,elem.value)){return elem.value}return null}function parseQuery(userQuery){function convertTypeFilterOnElem(elem){if(elem.typeFilter!==null){let typeFilter=elem.typeFilter;if(typeFilter==="const"){typeFilter="constant"}elem.typeFilter=itemTypeFromName(typeFilter)}else{elem.typeFilter=NO_TYPE_FILTER}for(const elem2 of elem.generics){convertTypeFilterOnElem(elem2)}}userQuery=userQuery.trim();const parserState={length:userQuery.length,pos:0,totalElems:0,genericsElems:0,typeFilter:null,userQuery:userQuery.toLowerCase(),};let query=newParsedQuery(userQuery);try{parseInput(query,parserState);for(const elem of query.elems){convertTypeFilterOnElem(elem)}for(const elem of query.returned){convertTypeFilterOnElem(elem)}}catch(err){query=newParsedQuery(userQuery);query.error=err;return query}if(!query.literalSearch){query.literalSearch=parserState.totalElems>1}query.foundElems=query.elems.length+query.returned.length;return query}function createQueryResults(results_in_args,results_returned,results_others,parsedQuery){return{"in_args":results_in_args,"returned":results_returned,"others":results_others,"query":parsedQuery,}}function execQuery(parsedQuery,searchWords,filterCrates,currentCrate){const results_others={},results_in_args={},results_returned={};function transformResults(results){const duplicates={};const out=[];for(const result of results){if(result.id>-1){const obj=searchIndex[result.id];obj.dist=result.dist;const res=buildHrefAndPath(obj);obj.displayPath=pathSplitter(res[0]);obj.fullPath=obj.displayPath+obj.name;obj.fullPath+="|"+obj.ty;if(duplicates[obj.fullPath]){continue}duplicates[obj.fullPath]=true;obj.href=res[1];out.push(obj);if(out.length>=MAX_RESULTS){break}}}return out}function sortResults(results,isType,preferredCrate){const userQuery=parsedQuery.userQuery;const ar=[];for(const entry in results){if(hasOwnPropertyRustdoc(results,entry)){const result=results[entry];result.word=searchWords[result.id];result.item=searchIndex[result.id]||{};ar.push(result)}}results=ar;if(results.length===0){return[]}results.sort((aaa,bbb)=>{let a,b;a=(aaa.word!==userQuery);b=(bbb.word!==userQuery);if(a!==b){return a-b}a=(aaa.index<0);b=(bbb.index<0);if(a!==b){return a-b}a=aaa.path_dist;b=bbb.path_dist;if(a!==b){return a-b}a=aaa.index;b=bbb.index;if(a!==b){return a-b}a=(aaa.dist);b=(bbb.dist);if(a!==b){return a-b}a=aaa.item.deprecated;b=bbb.item.deprecated;if(a!==b){return a-b}a=(aaa.item.crate!==preferredCrate);b=(bbb.item.crate!==preferredCrate);if(a!==b){return a-b}a=aaa.word.length;b=bbb.word.length;if(a!==b){return a-b}a=aaa.word;b=bbb.word;if(a!==b){return(a>b?+1:-1)}if((aaa.item.ty===TY_PRIMITIVE&&bbb.item.ty!==TY_KEYWORD)||(aaa.item.ty===TY_KEYWORD&&bbb.item.ty!==TY_PRIMITIVE)){return-1}if((bbb.item.ty===TY_PRIMITIVE&&aaa.item.ty!==TY_PRIMITIVE)||(bbb.item.ty===TY_KEYWORD&&aaa.item.ty!==TY_KEYWORD)){return 1}a=(aaa.item.desc==="");b=(bbb.item.desc==="");if(a!==b){return a-b}a=aaa.item.ty;b=bbb.item.ty;if(a!==b){return a-b}a=aaa.item.path;b=bbb.item.path;if(a!==b){return(a>b?+1:-1)}return 0});let nameSplit=null;if(parsedQuery.elems.length===1){const hasPath=typeof parsedQuery.elems[0].path==="undefined";nameSplit=hasPath?null:parsedQuery.elems[0].path}for(const result of results){if(result.dontValidate){continue}const name=result.item.name.toLowerCase(),path=result.item.path.toLowerCase(),parent=result.item.parent;if(!isType&&!validateResult(name,path,nameSplit,parent)){result.id=-1}}return transformResults(results)}function checkGenerics(row,elem,defaultDistance,maxEditDistance){if(row.generics.length===0){return elem.generics.length===0?defaultDistance:maxEditDistance+1}else if(row.generics.length>0&&row.generics[0].name===null){return checkGenerics(row.generics[0],elem,defaultDistance,maxEditDistance)}if(elem.generics.length>0&&row.generics.length>=elem.generics.length){const elems=Object.create(null);for(const entry of row.generics){if(entry.name===""){if(checkGenerics(entry,elem,maxEditDistance+1,maxEditDistance)!==0){return maxEditDistance+1}continue}if(elems[entry.name]===undefined){elems[entry.name]=[]}elems[entry.name].push(entry.ty)}const handleGeneric=generic=>{let match=null;if(elems[generic.name]){match=generic.name}else{for(const elem_name in elems){if(!hasOwnPropertyRustdoc(elems,elem_name)){continue}if(elem_name===generic){match=elem_name;break}}}if(match===null){return false}const matchIdx=elems[match].findIndex(tmp_elem=>typePassesFilter(generic.typeFilter,tmp_elem));if(matchIdx===-1){return false}elems[match].splice(matchIdx,1);if(elems[match].length===0){delete elems[match]}return true};for(const generic of elem.generics){if(generic.typeFilter!==-1&&!handleGeneric(generic)){return maxEditDistance+1}}for(const generic of elem.generics){if(generic.typeFilter===-1&&!handleGeneric(generic)){return maxEditDistance+1}}return 0}return maxEditDistance+1}function checkIfInGenerics(row,elem,maxEditDistance){let dist=maxEditDistance+1;for(const entry of row.generics){dist=Math.min(checkType(entry,elem,true,maxEditDistance),dist);if(dist===0){break}}return dist}function checkType(row,elem,literalSearch,maxEditDistance){if(row.name===null){if(row.generics.length>0){return checkIfInGenerics(row,elem,maxEditDistance)}return maxEditDistance+1}let dist;if(typePassesFilter(elem.typeFilter,row.ty)){dist=editDistance(row.name,elem.name,maxEditDistance)}else{dist=maxEditDistance+1}if(literalSearch){if(dist!==0){if(elem.generics.length===0){const checkGeneric=row.generics.length>0;if(checkGeneric&&row.generics.findIndex(tmp_elem=>tmp_elem.name===elem.name&&typePassesFilter(elem.typeFilter,tmp_elem.ty))!==-1){return 0}}return maxEditDistance+1}else if(elem.generics.length>0){return checkGenerics(row,elem,maxEditDistance+1,maxEditDistance)}return 0}else if(row.generics.length>0){if(elem.generics.length===0){if(dist===0){return 0}dist=Math.min(dist,checkIfInGenerics(row,elem,maxEditDistance));return dist}else if(dist>maxEditDistance){return checkIfInGenerics(row,elem,maxEditDistance)}else{const tmp_dist=checkGenerics(row,elem,dist,maxEditDistance);if(tmp_dist>maxEditDistance){return maxEditDistance+1}return(tmp_dist+dist)/2}}else if(elem.generics.length>0){return maxEditDistance+1}return dist}function findArg(row,elem,maxEditDistance,skipPositions){let dist=maxEditDistance+1;let position=-1;if(row&&row.type&&row.type.inputs&&row.type.inputs.length>0){let i=0;for(const input of row.type.inputs){if(skipPositions.indexOf(i)!==-1){i+=1;continue}const typeDist=checkType(input,elem,parsedQuery.literalSearch,maxEditDistance);if(typeDist===0){return{dist:0,position:i}}if(typeDist<dist){dist=typeDist;position=i}i+=1}}dist=parsedQuery.literalSearch?maxEditDistance+1:dist;return{dist,position}}function checkReturned(row,elem,maxEditDistance,skipPositions){let dist=maxEditDistance+1;let position=-1;if(row&&row.type&&row.type.output.length>0){const ret=row.type.output;let i=0;for(const ret_ty of ret){if(skipPositions.indexOf(i)!==-1){i+=1;continue}const typeDist=checkType(ret_ty,elem,parsedQuery.literalSearch,maxEditDistance);if(typeDist===0){return{dist:0,position:i}}if(typeDist<dist){dist=typeDist;position=i}i+=1}}dist=parsedQuery.literalSearch?maxEditDistance+1:dist;return{dist,position}}function checkPath(contains,ty,maxEditDistance){if(contains.length===0){return 0}let ret_dist=maxEditDistance+1;const path=ty.path.split("::");if(ty.parent&&ty.parent.name){path.push(ty.parent.name.toLowerCase())}const length=path.length;const clength=contains.length;if(clength>length){return maxEditDistance+1}for(let i=0;i<length;++i){if(i+clength>length){break}let dist_total=0;let aborted=false;for(let x=0;x<clength;++x){const dist=editDistance(path[i+x],contains[x],maxEditDistance);if(dist>maxEditDistance){aborted=true;break}dist_total+=dist}if(!aborted){ret_dist=Math.min(ret_dist,Math.round(dist_total/clength))}}return ret_dist}function typePassesFilter(filter,type){if(filter<=NO_TYPE_FILTER||filter===type)return true;const name=itemTypes[type];switch(itemTypes[filter]){case"constant":return name==="associatedconstant";case"fn":return name==="method"||name==="tymethod";case"type":return name==="primitive"||name==="associatedtype";case"trait":return name==="traitalias"}return false}function createAliasFromItem(item){return{crate:item.crate,name:item.name,path:item.path,desc:item.desc,ty:item.ty,parent:item.parent,type:item.type,is_alias:true,deprecated:item.deprecated,}}function handleAliases(ret,query,filterCrates,currentCrate){const lowerQuery=query.toLowerCase();const aliases=[];const crateAliases=[];if(filterCrates!==null){if(ALIASES[filterCrates]&&ALIASES[filterCrates][lowerQuery]){const query_aliases=ALIASES[filterCrates][lowerQuery];for(const alias of query_aliases){aliases.push(createAliasFromItem(searchIndex[alias]))}}}else{Object.keys(ALIASES).forEach(crate=>{if(ALIASES[crate][lowerQuery]){const pushTo=crate===currentCrate?crateAliases:aliases;const query_aliases=ALIASES[crate][lowerQuery];for(const alias of query_aliases){pushTo.push(createAliasFromItem(searchIndex[alias]))}}})}const sortFunc=(aaa,bbb)=>{if(aaa.path<bbb.path){return 1}else if(aaa.path===bbb.path){return 0}return-1};crateAliases.sort(sortFunc);aliases.sort(sortFunc);const pushFunc=alias=>{alias.alias=query;const res=buildHrefAndPath(alias);alias.displayPath=pathSplitter(res[0]);alias.fullPath=alias.displayPath+alias.name;alias.href=res[1];ret.others.unshift(alias);if(ret.others.length>MAX_RESULTS){ret.others.pop()}};aliases.forEach(pushFunc);crateAliases.forEach(pushFunc)}function addIntoResults(results,fullId,id,index,dist,path_dist,maxEditDistance){const inBounds=dist<=maxEditDistance||index!==-1;if(dist===0||(!parsedQuery.literalSearch&&inBounds)){if(results[fullId]!==undefined){const result=results[fullId];if(result.dontValidate||result.dist<=dist){return}}results[fullId]={id:id,index:index,dontValidate:parsedQuery.literalSearch,dist:dist,path_dist:path_dist,}}}function handleSingleArg(row,pos,elem,results_others,results_in_args,results_returned,maxEditDistance){if(!row||(filterCrates!==null&&row.crate!==filterCrates)){return}let dist,index=-1,path_dist=0;const fullId=row.id;const searchWord=searchWords[pos];const in_args=findArg(row,elem,maxEditDistance,[]);const returned=checkReturned(row,elem,maxEditDistance,[]);addIntoResults(results_in_args,fullId,pos,-1,in_args.dist,0,maxEditDistance);addIntoResults(results_returned,fullId,pos,-1,returned.dist,0,maxEditDistance);if(!typePassesFilter(elem.typeFilter,row.ty)){return}const row_index=row.normalizedName.indexOf(elem.pathLast);const word_index=searchWord.indexOf(elem.pathLast);if(row_index===-1){index=word_index}else if(word_index===-1){index=row_index}else if(word_index<row_index){index=word_index}else{index=row_index}if(elem.name.length===0){if(row.type!==null){dist=checkGenerics(row.type,elem,maxEditDistance+1,maxEditDistance);addIntoResults(results_others,fullId,pos,index,dist,0,maxEditDistance)}return}if(elem.fullPath.length>1){path_dist=checkPath(elem.pathWithoutLast,row,maxEditDistance);if(path_dist>maxEditDistance){return}}if(parsedQuery.literalSearch){if(searchWord===elem.name){addIntoResults(results_others,fullId,pos,index,0,path_dist)}return}dist=editDistance(searchWord,elem.pathLast,maxEditDistance);if(index===-1&&dist+path_dist>maxEditDistance){return}addIntoResults(results_others,fullId,pos,index,dist,path_dist,maxEditDistance)}function handleArgs(row,pos,results,maxEditDistance){if(!row||(filterCrates!==null&&row.crate!==filterCrates)){return}let totalDist=0;let nbDist=0;function checkArgs(elems,callback){const skipPositions=[];for(const elem of elems){const{dist,position}=callback(row,elem,maxEditDistance,skipPositions);if(dist<=1){nbDist+=1;totalDist+=dist;skipPositions.push(position)}else{return false}}return true}if(!checkArgs(parsedQuery.elems,findArg)){return}if(!checkArgs(parsedQuery.returned,checkReturned)){return}if(nbDist===0){return}const dist=Math.round(totalDist/nbDist);addIntoResults(results,row.id,pos,0,dist,0,maxEditDistance)}function innerRunQuery(){let elem,i,nSearchWords,in_returned,row;let queryLen=0;for(const elem of parsedQuery.elems){queryLen+=elem.name.length}for(const elem of parsedQuery.returned){queryLen+=elem.name.length}const maxEditDistance=Math.floor(queryLen/3);if(parsedQuery.foundElems===1){if(parsedQuery.elems.length===1){elem=parsedQuery.elems[0];for(i=0,nSearchWords=searchWords.length;i<nSearchWords;++i){handleSingleArg(searchIndex[i],i,elem,results_others,results_in_args,results_returned,maxEditDistance)}}else if(parsedQuery.returned.length===1){elem=parsedQuery.returned[0];for(i=0,nSearchWords=searchWords.length;i<nSearchWords;++i){row=searchIndex[i];in_returned=checkReturned(row,elem,maxEditDistance,[]);addIntoResults(results_others,row.id,i,-1,in_returned.dist,maxEditDistance)}}}else if(parsedQuery.foundElems>0){for(i=0,nSearchWords=searchWords.length;i<nSearchWords;++i){handleArgs(searchIndex[i],i,results_others,maxEditDistance)}}}if(parsedQuery.error===null){innerRunQuery()}const ret=createQueryResults(sortResults(results_in_args,true,currentCrate),sortResults(results_returned,true,currentCrate),sortResults(results_others,false,currentCrate),parsedQuery);handleAliases(ret,parsedQuery.original.replace(/"/g,""),filterCrates,currentCrate);if(parsedQuery.error!==null&&ret.others.length!==0){ret.query.error=null}return ret}function validateResult(name,path,keys,parent,maxEditDistance){if(!keys||!keys.length){return true}for(const key of keys){if(!(name.indexOf(key)>-1||path.indexOf(key)>-1||(parent!==undefined&&parent.name!==undefined&&parent.name.toLowerCase().indexOf(key)>-1)||editDistance(name,key,maxEditDistance)<=maxEditDistance)){return false}}return true}function nextTab(direction){const next=(searchState.currentTab+direction+3)%searchState.focusedByTab.length;searchState.focusedByTab[searchState.currentTab]=document.activeElement;printTab(next);focusSearchResult()}function focusSearchResult(){const target=searchState.focusedByTab[searchState.currentTab]||document.querySelectorAll(".search-results.active a").item(0)||document.querySelectorAll("#search-tabs button").item(searchState.currentTab);searchState.focusedByTab[searchState.currentTab]=null;if(target){target.focus()}}function buildHrefAndPath(item){let displayPath;let href;const type=itemTypes[item.ty];const name=item.name;let path=item.path;if(type==="mod"){displayPath=path+"::";href=ROOT_PATH+path.replace(/::/g,"/")+"/"+name+"/index.html"}else if(type==="import"){displayPath=item.path+"::";href=ROOT_PATH+item.path.replace(/::/g,"/")+"/index.html#reexport."+name}else if(type==="primitive"||type==="keyword"){displayPath="";href=ROOT_PATH+path.replace(/::/g,"/")+"/"+type+"."+name+".html"}else if(type==="externcrate"){displayPath="";href=ROOT_PATH+name+"/index.html"}else if(item.parent!==undefined){const myparent=item.parent;let anchor="#"+type+"."+name;const parentType=itemTypes[myparent.ty];let pageType=parentType;let pageName=myparent.name;if(parentType==="primitive"){displayPath=myparent.name+"::"}else if(type==="structfield"&&parentType==="variant"){const enumNameIdx=item.path.lastIndexOf("::");const enumName=item.path.substr(enumNameIdx+2);path=item.path.substr(0,enumNameIdx);displayPath=path+"::"+enumName+"::"+myparent.name+"::";anchor="#variant."+myparent.name+".field."+name;pageType="enum";pageName=enumName}else{displayPath=path+"::"+myparent.name+"::"}href=ROOT_PATH+path.replace(/::/g,"/")+"/"+pageType+"."+pageName+".html"+anchor}else{displayPath=item.path+"::";href=ROOT_PATH+item.path.replace(/::/g,"/")+"/"+type+"."+name+".html"}return[displayPath,href]}function pathSplitter(path){const tmp="<span>"+path.replace(/::/g,"::</span><span>");if(tmp.endsWith("<span>")){return tmp.slice(0,tmp.length-6)}return tmp}function addTab(array,query,display){let extraClass="";if(display===true){extraClass=" active"}const output=document.createElement("div");let length=0;if(array.length>0){output.className="search-results "+extraClass;array.forEach(item=>{const name=item.name;const type=itemTypes[item.ty];length+=1;let extra="";if(type==="primitive"){extra=" <i>(primitive type)</i>"}else if(type==="keyword"){extra=" <i>(keyword)</i>"}const link=document.createElement("a");link.className="result-"+type;link.href=item.href;const resultName=document.createElement("div");resultName.className="result-name";if(item.is_alias){const alias=document.createElement("span");alias.className="alias";const bold=document.createElement("b");bold.innerText=item.alias;alias.appendChild(bold);alias.insertAdjacentHTML("beforeend","<span class=\"grey\"><i>&nbsp;- see&nbsp;</i></span>");resultName.appendChild(alias)}resultName.insertAdjacentHTML("beforeend",item.displayPath+"<span class=\""+type+"\">"+name+extra+"</span>");link.appendChild(resultName);const description=document.createElement("div");description.className="desc";description.insertAdjacentHTML("beforeend",item.desc);link.appendChild(description);output.appendChild(link)})}else if(query.error===null){output.className="search-failed"+extraClass;output.innerHTML="No results :(<br/>"+"Try on <a href=\"https://duckduckgo.com/?q="+encodeURIComponent("rust "+query.userQuery)+"\">DuckDuckGo</a>?<br/><br/>"+"Or try looking in one of these:<ul><li>The <a "+"href=\"https://doc.rust-lang.org/reference/index.html\">Rust Reference</a> "+" for technical details about the language.</li><li><a "+"href=\"https://doc.rust-lang.org/rust-by-example/index.html\">Rust By "+"Example</a> for expository code examples.</a></li><li>The <a "+"href=\"https://doc.rust-lang.org/book/index.html\">Rust Book</a> for "+"introductions to language features and the language itself.</li><li><a "+"href=\"https://docs.rs\">Docs.rs</a> for documentation of crates released on"+" <a href=\"https://crates.io/\">crates.io</a>.</li></ul>"}return[output,length]}function makeTabHeader(tabNb,text,nbElems){if(searchState.currentTab===tabNb){return"<button class=\"selected\">"+text+" <span class=\"count\">("+nbElems+")</span></button>"}return"<button>"+text+" <span class=\"count\">("+nbElems+")</span></button>"}function showResults(results,go_to_first,filterCrates){const search=searchState.outputElement();if(go_to_first||(results.others.length===1&&getSettingValue("go-to-only-result")==="true")){const elem=document.createElement("a");elem.href=results.others[0].href;removeClass(elem,"active");document.body.appendChild(elem);elem.click();return}if(results.query===undefined){results.query=parseQuery(searchState.input.value)}currentResults=results.query.userQuery;const ret_others=addTab(results.others,results.query,true);const ret_in_args=addTab(results.in_args,results.query,false);const ret_returned=addTab(results.returned,results.query,false);let currentTab=searchState.currentTab;if((currentTab===0&&ret_others[1]===0)||(currentTab===1&&ret_in_args[1]===0)||(currentTab===2&&ret_returned[1]===0)){if(ret_others[1]!==0){currentTab=0}else if(ret_in_args[1]!==0){currentTab=1}else if(ret_returned[1]!==0){currentTab=2}}let crates="";const crates_list=Object.keys(rawSearchIndex);if(crates_list.length>1){crates=" in&nbsp;<div id=\"crate-search-div\"><select id=\"crate-search\">"+"<option value=\"all crates\">all crates</option>";for(const c of crates_list){crates+=`<option value="${c}" ${c === filterCrates && "selected"}>${c}</option>`}crates+="</select></div>"}let output=`<h1 class="search-results-title">Results${crates}</h1>`;if(results.query.error!==null){const error=results.query.error;error.forEach((value,index)=>{value=value.split("<").join("&lt;").split(">").join("&gt;");if(index%2!==0){error[index]=`<code>${value}</code>`}else{error[index]=value}});output+=`<h3 class="error">Query parser error: "${error.join("")}".</h3>`;output+="<div id=\"search-tabs\">"+makeTabHeader(0,"In Names",ret_others[1])+"</div>";currentTab=0}else if(results.query.foundElems<=1&&results.query.returned.length===0){output+="<div id=\"search-tabs\">"+makeTabHeader(0,"In Names",ret_others[1])+makeTabHeader(1,"In Parameters",ret_in_args[1])+makeTabHeader(2,"In Return Types",ret_returned[1])+"</div>"}else{const signatureTabTitle=results.query.elems.length===0?"In Function Return Types":results.query.returned.length===0?"In Function Parameters":"In Function Signatures";output+="<div id=\"search-tabs\">"+makeTabHeader(0,signatureTabTitle,ret_others[1])+"</div>";currentTab=0}const resultsElem=document.createElement("div");resultsElem.id="results";resultsElem.appendChild(ret_others[0]);resultsElem.appendChild(ret_in_args[0]);resultsElem.appendChild(ret_returned[0]);search.innerHTML=output;const crateSearch=document.getElementById("crate-search");if(crateSearch){crateSearch.addEventListener("input",updateCrate)}search.appendChild(resultsElem);searchState.showResults(search);const elems=document.getElementById("search-tabs").childNodes;searchState.focusedByTab=[];let i=0;for(const elem of elems){const j=i;elem.onclick=()=>printTab(j);searchState.focusedByTab.push(null);i+=1}printTab(currentTab)}function search(e,forced){if(e){e.preventDefault()}const query=parseQuery(searchState.input.value.trim());let filterCrates=getFilterCrates();if(!forced&&query.userQuery===currentResults){if(query.userQuery.length>0){putBackSearch()}return}searchState.setLoadingSearch();const params=searchState.getQueryStringParams();if(filterCrates===null&&params["filter-crate"]!==undefined){filterCrates=params["filter-crate"]}searchState.title="Results for "+query.original+" - Rust";if(browserSupportsHistoryApi()){const newURL=buildUrl(query.original,filterCrates);if(!history.state&&!params.search){history.pushState(null,"",newURL)}else{history.replaceState(null,"",newURL)}}showResults(execQuery(query,searchWords,filterCrates,window.currentCrate),params.go_to_first,filterCrates)}function buildItemSearchTypeAll(types,lowercasePaths){const PATH_INDEX_DATA=0;const GENERICS_DATA=1;return types.map(type=>{let pathIndex,generics;if(typeof type==="number"){pathIndex=type;generics=[]}else{pathIndex=type[PATH_INDEX_DATA];generics=buildItemSearchTypeAll(type[GENERICS_DATA],lowercasePaths)}return{name:pathIndex===0?null:lowercasePaths[pathIndex-1].name,ty:pathIndex===0?null:lowercasePaths[pathIndex-1].ty,generics:generics,}})}function buildFunctionSearchType(functionSearchType,lowercasePaths){const INPUTS_DATA=0;const OUTPUT_DATA=1;if(functionSearchType===0){return null}let inputs,output;if(typeof functionSearchType[INPUTS_DATA]==="number"){const pathIndex=functionSearchType[INPUTS_DATA];inputs=[{name:pathIndex===0?null:lowercasePaths[pathIndex-1].name,ty:pathIndex===0?null:lowercasePaths[pathIndex-1].ty,generics:[],}]}else{inputs=buildItemSearchTypeAll(functionSearchType[INPUTS_DATA],lowercasePaths)}if(functionSearchType.length>1){if(typeof functionSearchType[OUTPUT_DATA]==="number"){const pathIndex=functionSearchType[OUTPUT_DATA];output=[{name:pathIndex===0?null:lowercasePaths[pathIndex-1].name,ty:pathIndex===0?null:lowercasePaths[pathIndex-1].ty,generics:[],}]}else{output=buildItemSearchTypeAll(functionSearchType[OUTPUT_DATA],lowercasePaths)}}else{output=[]}return{inputs,output,}}function buildIndex(rawSearchIndex){searchIndex=[];const searchWords=[];const charA="A".charCodeAt(0);let currentIndex=0;let id=0;for(const crate in rawSearchIndex){if(!hasOwnPropertyRustdoc(rawSearchIndex,crate)){continue}let crateSize=0;const crateCorpus=rawSearchIndex[crate];searchWords.push(crate);const crateRow={crate:crate,ty:1,name:crate,path:"",desc:crateCorpus.doc,parent:undefined,type:null,id:id,normalizedName:crate.indexOf("_")===-1?crate:crate.replace(/_/g,""),deprecated:null,};id+=1;searchIndex.push(crateRow);currentIndex+=1;const itemTypes=crateCorpus.t;const itemNames=crateCorpus.n;const itemPaths=new Map(crateCorpus.q);const itemDescs=crateCorpus.d;const itemParentIdxs=crateCorpus.i;const itemFunctionSearchTypes=crateCorpus.f;const deprecatedItems=new Set(crateCorpus.c);const paths=crateCorpus.p;const aliases=crateCorpus.a;const lowercasePaths=[];let len=paths.length;for(let i=0;i<len;++i){lowercasePaths.push({ty:paths[i][0],name:paths[i][1].toLowerCase()});paths[i]={ty:paths[i][0],name:paths[i][1]}}len=itemTypes.length;let lastPath="";for(let i=0;i<len;++i){let word="";if(typeof itemNames[i]==="string"){word=itemNames[i].toLowerCase()}searchWords.push(word);const row={crate:crate,ty:itemTypes.charCodeAt(i)-charA,name:itemNames[i],path:itemPaths.has(i)?itemPaths.get(i):lastPath,desc:itemDescs[i],parent:itemParentIdxs[i]>0?paths[itemParentIdxs[i]-1]:undefined,type:buildFunctionSearchType(itemFunctionSearchTypes[i],lowercasePaths),id:id,normalizedName:word.indexOf("_")===-1?word:word.replace(/_/g,""),deprecated:deprecatedItems.has(i),};id+=1;searchIndex.push(row);lastPath=row.path;crateSize+=1}if(aliases){ALIASES[crate]=Object.create(null);for(const alias_name in aliases){if(!hasOwnPropertyRustdoc(aliases,alias_name)){continue}if(!hasOwnPropertyRustdoc(ALIASES[crate],alias_name)){ALIASES[crate][alias_name]=[]}for(const local_alias of aliases[alias_name]){ALIASES[crate][alias_name].push(local_alias+currentIndex)}}}currentIndex+=crateSize}return searchWords}function onSearchSubmit(e){e.preventDefault();searchState.clearInputTimeout();search()}function putBackSearch(){const search_input=searchState.input;if(!searchState.input){return}if(search_input.value!==""&&!searchState.isDisplayed()){searchState.showResults();if(browserSupportsHistoryApi()){history.replaceState(null,"",buildUrl(search_input.value,getFilterCrates()))}document.title=searchState.title}}function registerSearchEvents(){const params=searchState.getQueryStringParams();if(searchState.input.value===""){searchState.input.value=params.search||""}const searchAfter500ms=()=>{searchState.clearInputTimeout();if(searchState.input.value.length===0){if(browserSupportsHistoryApi()){history.replaceState(null,window.currentCrate+" - Rust",getNakedUrl()+window.location.hash)}searchState.hideResults()}else{searchState.timeout=setTimeout(search,500)}};searchState.input.onkeyup=searchAfter500ms;searchState.input.oninput=searchAfter500ms;document.getElementsByClassName("search-form")[0].onsubmit=onSearchSubmit;searchState.input.onchange=e=>{if(e.target!==document.activeElement){return}searchState.clearInputTimeout();setTimeout(search,0)};searchState.input.onpaste=searchState.input.onchange;searchState.outputElement().addEventListener("keydown",e=>{if(e.altKey||e.ctrlKey||e.shiftKey||e.metaKey){return}if(e.which===38){const previous=document.activeElement.previousElementSibling;if(previous){previous.focus()}else{searchState.focus()}e.preventDefault()}else if(e.which===40){const next=document.activeElement.nextElementSibling;if(next){next.focus()}const rect=document.activeElement.getBoundingClientRect();if(window.innerHeight-rect.bottom<rect.height){window.scrollBy(0,rect.height)}e.preventDefault()}else if(e.which===37){nextTab(-1);e.preventDefault()}else if(e.which===39){nextTab(1);e.preventDefault()}});searchState.input.addEventListener("keydown",e=>{if(e.which===40){focusSearchResult();e.preventDefault()}});searchState.input.addEventListener("focus",()=>{putBackSearch()});searchState.input.addEventListener("blur",()=>{searchState.input.placeholder=searchState.input.origPlaceholder});if(browserSupportsHistoryApi()){const previousTitle=document.title;window.addEventListener("popstate",e=>{const params=searchState.getQueryStringParams();document.title=previousTitle;currentResults=null;if(params.search&&params.search.length>0){searchState.input.value=params.search;search(e)}else{searchState.input.value="";searchState.hideResults()}})}window.onpageshow=()=>{const qSearch=searchState.getQueryStringParams().search;if(searchState.input.value===""&&qSearch){searchState.input.value=qSearch}search()}}function updateCrate(ev){if(ev.target.value==="all crates"){const params=searchState.getQueryStringParams();const query=searchState.input.value.trim();if(!history.state&&!params.search){history.pushState(null,"",buildUrl(query,null))}else{history.replaceState(null,"",buildUrl(query,null))}}currentResults=null;search(undefined,true)}const searchWords=buildIndex(rawSearchIndex);if(typeof window!=="undefined"){registerSearchEvents();if(window.searchState.getQueryStringParams().search){search()}}if(typeof exports!=="undefined"){exports.initSearch=initSearch;exports.execQuery=execQuery;exports.parseQuery=parseQuery}return searchWords}if(typeof window!=="undefined"){window.initSearch=initSearch;if(window.searchIndex!==undefined){initSearch(window.searchIndex)}}else{initSearch({})}})()