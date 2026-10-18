// Translated format templates; indices preserve argument order.
#[macro_export]
macro_rules! localized_format {
    ("播放 {0} 次" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("播放 {0} 次" $(, $arg)*),
        2 => format!("Play {0} times" $(, $arg)*),
        3 => format!("{0}回再生" $(, $arg)*),
        4 => format!("{0}회 재생" $(, $arg)*),
        5 => format!("{0}-mal abspielen" $(, $arg)*),
        6 => format!("Lire {0} fois" $(, $arg)*),
        _ => format!("播放 {0} 次" $(, $arg)*),
    }};
    ("帧 {0} / {1} · {2} 秒/帧" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("影格 {0} / {1} · {2} 秒/影格" $(, $arg)*),
        2 => format!("Frame {0} / {1} · {2} s/frame" $(, $arg)*),
        3 => format!("フレーム {0} / {1} · {2} 秒/フレーム" $(, $arg)*),
        4 => format!("프레임 {0} / {1} · {2} 초/프레임" $(, $arg)*),
        5 => format!("Bild {0} / {1} · {2} s/Bild" $(, $arg)*),
        6 => format!("Image {0} / {1} · {2} s/image" $(, $arg)*),
        _ => format!("帧 {0} / {1} · {2} 秒/帧" $(, $arg)*),
    }};

    ("扫描中有 {0} 项无法读取或存在重复 GUID；相关字段可能无法确定类型。" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("掃描中有 {0} 項無法讀取或 GUID 重複；相關欄位類型可能無法確定。" $(, $arg)*),
        2 => format!("{0} unreadable items or duplicate GUIDs; some field types may be unknown." $(, $arg)*),
        3 => format!("読み取り不可または GUID 重複が {0} 件。型を特定できない場合があります。" $(, $arg)*),
        4 => format!("읽기 실패 또는 중복 GUID {0}개; 일부 필드 유형을 알 수 없습니다." $(, $arg)*),
        5 => format!("{0} unlesbare Einträge oder doppelte GUIDs; einige Feldtypen sind eventuell unbekannt." $(, $arg)*),
        6 => format!("{0} éléments illisibles ou GUID dupliqués ; certains types peuvent être inconnus." $(, $arg)*),
        _ => format!("扫描中有 {0} 项无法读取或存在重复 GUID；相关字段可能无法确定类型。" $(, $arg)*),
    }};
    ("Unity 工程：{0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("Unity 專案：{0}" $(, $arg)*),
        2 => format!("Unity project: {0}" $(, $arg)*),
        3 => format!("Unity プロジェクト：{0}" $(, $arg)*),
        4 => format!("Unity 프로젝트: {0}" $(, $arg)*),
        5 => format!("Unity-Projekt: {0}" $(, $arg)*),
        6 => format!("Projet Unity : {0}" $(, $arg)*),
        _ => format!("Unity 工程：{0}" $(, $arg)*),
    }};
    ("脚本：{0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("腳本：{0}" $(, $arg)*),
        2 => format!("Script: {0}" $(, $arg)*),
        3 => format!("スクリプト：{0}" $(, $arg)*),
        4 => format!("스크립트: {0}" $(, $arg)*),
        5 => format!("Skript: {0}" $(, $arg)*),
        6 => format!("Script : {0}" $(, $arg)*),
        _ => format!("脚本：{0}" $(, $arg)*),
    }};
    ("引用文件不存在：{0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("引用檔案不存在：{0}" $(, $arg)*),
        2 => format!("Referenced file does not exist: {0}" $(, $arg)*),
        3 => format!("参照ファイルが存在しません：{0}" $(, $arg)*),
        4 => format!("참조 파일이 없습니다: {0}" $(, $arg)*),
        5 => format!("Referenzierte Datei fehlt: {0}" $(, $arg)*),
        6 => format!("Fichier référencé inexistant : {0}" $(, $arg)*),
        _ => format!("引用文件不存在：{0}" $(, $arg)*),
    }};
    ("已交给系统默认程序打开：{0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("已交由系統預設程式開啟：{0}" $(, $arg)*),
        2 => format!("Opened with the default application: {0}" $(, $arg)*),
        3 => format!("既定のアプリで開きました：{0}" $(, $arg)*),
        4 => format!("기본 앱으로 열었습니다: {0}" $(, $arg)*),
        5 => format!("Mit Standardanwendung geöffnet: {0}" $(, $arg)*),
        6 => format!("Ouvert avec l’application par défaut : {0}" $(, $arg)*),
        _ => format!("已交给系统默认程序打开：{0}" $(, $arg)*),
    }};
    ("无法打开引用：{0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("無法開啟引用：{0}" $(, $arg)*),
        2 => format!("Cannot open reference: {0}" $(, $arg)*),
        3 => format!("参照を開けません：{0}" $(, $arg)*),
        4 => format!("참조를 열 수 없습니다: {0}" $(, $arg)*),
        5 => format!("Referenz kann nicht geöffnet werden: {0}" $(, $arg)*),
        6 => format!("Impossible d’ouvrir la référence : {0}" $(, $arg)*),
        _ => format!("无法打开引用：{0}" $(, $arg)*),
    }};
    ("无法读取文件：{0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("無法讀取檔案：{0}" $(, $arg)*),
        2 => format!("Cannot read file: {0}" $(, $arg)*),
        3 => format!("ファイルを読み込めません：{0}" $(, $arg)*),
        4 => format!("파일을 읽을 수 없습니다: {0}" $(, $arg)*),
        5 => format!("Datei kann nicht gelesen werden: {0}" $(, $arg)*),
        6 => format!("Impossible de lire le fichier : {0}" $(, $arg)*),
        _ => format!("无法读取文件：{0}" $(, $arg)*),
    }};
    ("无法写入文件：{0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("無法寫入檔案：{0}" $(, $arg)*),
        2 => format!("Cannot write file: {0}" $(, $arg)*),
        3 => format!("ファイルに書き込めません：{0}" $(, $arg)*),
        4 => format!("파일에 쓸 수 없습니다: {0}" $(, $arg)*),
        5 => format!("Datei kann nicht geschrieben werden: {0}" $(, $arg)*),
        6 => format!("Impossible d’écrire le fichier : {0}" $(, $arg)*),
        _ => format!("无法写入文件：{0}" $(, $arg)*),
    }};
    ("无法进入编辑模式：{0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("無法進入編輯模式：{0}" $(, $arg)*),
        2 => format!("Cannot enter edit mode: {0}" $(, $arg)*),
        3 => format!("編集モードに切り替えられません：{0}" $(, $arg)*),
        4 => format!("편집 모드로 전환할 수 없습니다: {0}" $(, $arg)*),
        5 => format!("Bearbeitungsmodus nicht verfügbar: {0}" $(, $arg)*),
        6 => format!("Impossible de passer en mode édition : {0}" $(, $arg)*),
        _ => format!("无法进入编辑模式：{0}" $(, $arg)*),
    }};
    ("重载失败：{0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("重新載入失敗：{0}" $(, $arg)*),
        2 => format!("Reload failed: {0}" $(, $arg)*),
        3 => format!("再読み込みに失敗：{0}" $(, $arg)*),
        4 => format!("다시 불러오기 실패: {0}" $(, $arg)*),
        5 => format!("Neuladen fehlgeschlagen: {0}" $(, $arg)*),
        6 => format!("Échec du rechargement : {0}" $(, $arg)*),
        _ => format!("重载失败：{0}" $(, $arg)*),
    }};
    ("已读取 {0} 个 Unity 序列化对象（只读模式）" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("已讀取 {0} 個 Unity 序列化物件（唯讀模式）" $(, $arg)*),
        2 => format!("Loaded {0} Unity serialized objects (read-only)" $(, $arg)*),
        3 => format!("Unity シリアライズオブジェクトを {0} 個読み込み（読み取り専用）" $(, $arg)*),
        4 => format!("Unity 직렬화 객체 {0}개 읽음 (읽기 전용)" $(, $arg)*),
        5 => format!("{0} serialisierte Unity-Objekte geladen (schreibgeschützt)" $(, $arg)*),
        6 => format!("{0} objets Unity sérialisés chargés (lecture seule)" $(, $arg)*),
        _ => format!("已读取 {0} 个 Unity 序列化对象（只读模式）" $(, $arg)*),
    }};
    ("已从磁盘重载 {0} 个 Unity 序列化对象（只读模式），旧撤销记录已清空" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("已從磁碟重新載入 {0} 個 Unity 序列化物件（唯讀模式），舊復原紀錄已清除" $(, $arg)*),
        2 => format!("Reloaded {0} Unity objects from disk (read-only); undo history cleared" $(, $arg)*),
        3 => format!("ディスクから {0} 個の Unity オブジェクトを再読み込み（読み取り専用）。履歴を消去しました" $(, $arg)*),
        4 => format!("디스크에서 Unity 객체 {0}개 다시 읽음 (읽기 전용); 실행 취소 기록 삭제됨" $(, $arg)*),
        5 => format!("{0} Unity-Objekte neu geladen (schreibgeschützt); Rückgängig-Verlauf gelöscht" $(, $arg)*),
        6 => format!("{0} objets Unity rechargés (lecture seule) ; historique d’annulation effacé" $(, $arg)*),
        _ => format!("已从磁盘重载 {0} 个 Unity 序列化对象（只读模式），旧撤销记录已清空" $(, $arg)*),
    }};
    ("{0}  ·  fileID: {1}  ·  YAML 第 {2} 行" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("{0}  ·  fileID: {1}  ·  YAML 第 {2} 行" $(, $arg)*),
        2 => format!("{0}  ·  fileID: {1}  ·  YAML line {2}" $(, $arg)*),
        3 => format!("{0}  ·  fileID: {1}  ·  YAML {2} 行目" $(, $arg)*),
        4 => format!("{0}  ·  fileID: {1}  ·  YAML {2}행" $(, $arg)*),
        5 => format!("{0}  ·  fileID: {1}  ·  YAML-Zeile {2}" $(, $arg)*),
        6 => format!("{0}  ·  fileID: {1}  ·  ligne YAML {2}" $(, $arg)*),
        _ => format!("{0}  ·  fileID: {1}  ·  YAML 第 {2} 行" $(, $arg)*),
    }};
    ("复制 {0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("複製 {0}" $(, $arg)*),
        2 => format!("Copy {0}" $(, $arg)*),
        3 => format!("{0} をコピー" $(, $arg)*),
        4 => format!("{0} 복사" $(, $arg)*),
        5 => format!("{0} kopieren" $(, $arg)*),
        6 => format!("Copier {0}" $(, $arg)*),
        _ => format!("复制 {0}" $(, $arg)*),
    }};
    ("总时长：0.00s — {0:.2}s{1}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("總時長：0.00s — {0:.2}s{1}" $(, $arg)*),
        2 => format!("Duration: 0.00s — {0:.2}s{1}" $(, $arg)*),
        3 => format!("再生時間：0.00s — {0:.2}s{1}" $(, $arg)*),
        4 => format!("총 길이: 0.00s — {0:.2}s{1}" $(, $arg)*),
        5 => format!("Dauer: 0.00s — {0:.2}s{1}" $(, $arg)*),
        6 => format!("Durée : 0.00s — {0:.2}s{1}" $(, $arg)*),
        _ => format!("总时长：0.00s — {0:.2}s{1}" $(, $arg)*),
    }};
    ("{0}　·　文件编码：{1}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("{0}　·　檔案編碼：{1}" $(, $arg)*),
        2 => format!("{0} · Encoding: {1}" $(, $arg)*),
        3 => format!("{0} · 文字コード：{1}" $(, $arg)*),
        4 => format!("{0} · 파일 인코딩: {1}" $(, $arg)*),
        5 => format!("{0} · Dateikodierung: {1}" $(, $arg)*),
        6 => format!("{0} · Encodage : {1}" $(, $arg)*),
        _ => format!("{0}　·　文件编码：{1}" $(, $arg)*),
    }};
    ("本文件中未找到 fileID: {0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("本檔案中未找到 fileID: {0}" $(, $arg)*),
        2 => format!("fileID not found in this file: {0}" $(, $arg)*),
        3 => format!("このファイルに fileID が見つかりません：{0}" $(, $arg)*),
        4 => format!("이 파일에서 fileID를 찾을 수 없습니다: {0}" $(, $arg)*),
        5 => format!("fileID in dieser Datei nicht gefunden: {0}" $(, $arg)*),
        6 => format!("fileID introuvable dans ce fichier : {0}" $(, $arg)*),
        _ => format!("本文件中未找到 fileID: {0}" $(, $arg)*),
    }};
    ("版本 {0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("版本 {0}" $(, $arg)*),
        2 => format!("Version {0}" $(, $arg)*),
        3 => format!("バージョン {0}" $(, $arg)*),
        4 => format!("버전 {0}" $(, $arg)*),
        5 => format!("Version {0}" $(, $arg)*),
        6 => format!("Version {0}" $(, $arg)*),
        _ => format!("版本 {0}" $(, $arg)*),
    }};
    ("“{0}”有未保存的修改。是否放弃编辑并关闭文件？" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("「{0}」有未儲存的修改。是否放棄編輯並關閉檔案？" $(, $arg)*),
        2 => format!("“{0}” has unsaved changes. Discard edits and close?" $(, $arg)*),
        3 => format!("「{0}」に未保存の変更があります。破棄して閉じますか？" $(, $arg)*),
        4 => format!("“{0}”에 저장하지 않은 변경 사항이 있습니다. 취소하고 닫을까요?" $(, $arg)*),
        5 => format!("„{0}“ enthält ungespeicherte Änderungen. Verwerfen und schließen?" $(, $arg)*),
        6 => format!("« {0} » contient des modifications non enregistrées. Les abandonner et fermer ?" $(, $arg)*),
        _ => format!("“{0}”有未保存的修改。是否放弃编辑并关闭文件？" $(, $arg)*),
    }};
    ("当前属性：{0} · {1}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("目前屬性：{0} · {1}" $(, $arg)*),
        2 => format!("Current property: {0} · {1}" $(, $arg)*),
        3 => format!("現在のプロパティ：{0} · {1}" $(, $arg)*),
        4 => format!("현재 속성: {0} · {1}" $(, $arg)*),
        5 => format!("Aktuelle Eigenschaft: {0} · {1}" $(, $arg)*),
        6 => format!("Propriété actuelle : {0} · {1}" $(, $arg)*),
        _ => format!("当前属性：{0} · {1}" $(, $arg)*),
    }};
    ("关键帧：{0}　·　时间：{1:.3}s" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("關鍵影格：{0}　·　時間：{1:.3}s" $(, $arg)*),
        2 => format!("Keyframe: {0} · Time: {1:.3}s" $(, $arg)*),
        3 => format!("キーフレーム：{0} · 時間：{1:.3}s" $(, $arg)*),
        4 => format!("키프레임: {0} · 시간: {1:.3}s" $(, $arg)*),
        5 => format!("Keyframe: {0} · Zeit: {1:.3}s" $(, $arg)*),
        6 => format!("Image clé : {0} · Temps : {1:.3}s" $(, $arg)*),
        _ => format!("关键帧：{0}　·　时间：{1:.3}s" $(, $arg)*),
    }};
    ("值：{0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("值：{0}" $(, $arg)*),
        2 => format!("Value: {0}" $(, $arg)*),
        3 => format!("値：{0}" $(, $arg)*),
        4 => format!("값: {0}" $(, $arg)*),
        5 => format!("Wert: {0}" $(, $arg)*),
        6 => format!("Valeur : {0}" $(, $arg)*),
        _ => format!("值：{0}" $(, $arg)*),
    }};
    ("{0}\n原始 YAML：{1}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("{0}\n原始 YAML：{1}" $(, $arg)*),
        2 => format!("{0}\nRaw YAML: {1}" $(, $arg)*),
        3 => format!("{0}\n元の YAML：{1}" $(, $arg)*),
        4 => format!("{0}\n원본 YAML: {1}" $(, $arg)*),
        5 => format!("{0}\nOriginal-YAML: {1}" $(, $arg)*),
        6 => format!("{0}\nYAML original : {1}" $(, $arg)*),
        _ => format!("{0}\n原始 YAML：{1}" $(, $arg)*),
    }};
    ("{0}（{1} 条）" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("{0}（{1} 筆）" $(, $arg)*),
        2 => format!("{0} ({1} entries)" $(, $arg)*),
        3 => format!("{0}（{1} 件）" $(, $arg)*),
        4 => format!("{0} ({1}개)" $(, $arg)*),
        5 => format!("{0} ({1} Einträge)" $(, $arg)*),
        6 => format!("{0} ({1} entrées)" $(, $arg)*),
        _ => format!("{0}（{1} 条）" $(, $arg)*),
    }};
    ("{0} ({2}) · {1} 条" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("{0} ({2}) · {1} 筆" $(, $arg)*),
        2 => format!("{0} ({2}) · {1} entries" $(, $arg)*),
        3 => format!("{0} ({2}) · {1} 件" $(, $arg)*),
        4 => format!("{0} ({2}) · {1}개" $(, $arg)*),
        5 => format!("{0} ({2}) · {1} Einträge" $(, $arg)*),
        6 => format!("{0} ({2}) · {1} entrées" $(, $arg)*),
        _ => format!("{0} ({2}) · {1} 条" $(, $arg)*),
    }};
    ("{0} ({1}[]) · {2} 条" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("{0} ({1}[]) · {2} 筆" $(, $arg)*),
        2 => format!("{0} ({1}[]) · {2} entries" $(, $arg)*),
        3 => format!("{0} ({1}[]) · {2} 件" $(, $arg)*),
        4 => format!("{0} ({1}[]) · {2}개" $(, $arg)*),
        5 => format!("{0} ({1}[]) · {2} Einträge" $(, $arg)*),
        6 => format!("{0} ({1}[]) · {2} entrées" $(, $arg)*),
        _ => format!("{0} ({1}[]) · {2} 条" $(, $arg)*),
    }};
    ("未匹配值 ({0})" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("未匹配值 ({0})" $(, $arg)*),
        2 => format!("Unmatched value ({0})" $(, $arg)*),
        3 => format!("一致しない値 ({0})" $(, $arg)*),
        4 => format!("일치하지 않는 값 ({0})" $(, $arg)*),
        5 => format!("Nicht zugeordneter Wert ({0})" $(, $arg)*),
        6 => format!("Valeur non correspondante ({0})" $(, $arg)*),
        _ => format!("未匹配值 ({0})" $(, $arg)*),
    }};
    ("状态：{0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("狀態：{0}" $(, $arg)*),
        2 => format!("State: {0}" $(, $arg)*),
        3 => format!("ステート：{0}" $(, $arg)*),
        4 => format!("상태: {0}" $(, $arg)*),
        5 => format!("Zustand: {0}" $(, $arg)*),
        6 => format!("État : {0}" $(, $arg)*),
        _ => format!("状态：{0}" $(, $arg)*),
    }};
    ("颜色关键点 {0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("顏色關鍵點 {0}" $(, $arg)*),
        2 => format!("Color key {0}" $(, $arg)*),
        3 => format!("カラーキー {0}" $(, $arg)*),
        4 => format!("색상 키 {0}" $(, $arg)*),
        5 => format!("Farbschlüssel {0}" $(, $arg)*),
        6 => format!("Clé de couleur {0}" $(, $arg)*),
        _ => format!("颜色关键点 {0}" $(, $arg)*),
    }};
    ("Alpha 关键点 {0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("Alpha 關鍵點 {0}" $(, $arg)*),
        2 => format!("Alpha key {0}" $(, $arg)*),
        3 => format!("アルファキー {0}" $(, $arg)*),
        4 => format!("알파 키 {0}" $(, $arg)*),
        5 => format!("Alpha-Schlüssel {0}" $(, $arg)*),
        6 => format!("Clé alpha {0}" $(, $arg)*),
        _ => format!("Alpha 关键点 {0}" $(, $arg)*),
    }};
    ("Points · {0} 条路径，{1} 个顶点" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("Points · {0} 條路徑，{1} 個頂點" $(, $arg)*),
        2 => format!("Points · {0} paths, {1} vertices" $(, $arg)*),
        3 => format!("Points · {0} パス、{1} 頂点" $(, $arg)*),
        4 => format!("Points · 경로 {0}개, 정점 {1}개" $(, $arg)*),
        5 => format!("Points · {0} Pfade, {1} Punkte" $(, $arg)*),
        6 => format!("Points · {0} chemins, {1} sommets" $(, $arg)*),
        _ => format!("Points · {0} 条路径，{1} 个顶点" $(, $arg)*),
    }};
    ("Path {1} · {0} 个顶点" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("Path {1} · {0} 個頂點" $(, $arg)*),
        2 => format!("Path {1} · {0} vertices" $(, $arg)*),
        3 => format!("Path {1} · {0} 頂点" $(, $arg)*),
        4 => format!("Path {1} · 정점 {0}개" $(, $arg)*),
        5 => format!("Path {1} · {0} Punkte" $(, $arg)*),
        6 => format!("Path {1} · {0} sommets" $(, $arg)*),
        _ => format!("Path {1} · {0} 个顶点" $(, $arg)*),
    }};
    ("RGB: {0}, {1}, {2}\nAlpha: {3}\n点击打开调色板（仅预览）" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("RGB: {0}, {1}, {2}\nAlpha: {3}\n點擊開啟調色盤（僅預覽）" $(, $arg)*),
        2 => format!("RGB: {0}, {1}, {2}\nAlpha: {3}\nClick to open palette (preview only)" $(, $arg)*),
        3 => format!("RGB: {0}, {1}, {2}\nAlpha: {3}\nクリックでパレットを開く（プレビューのみ）" $(, $arg)*),
        4 => format!("RGB: {0}, {1}, {2}\nAlpha: {3}\n클릭하여 팔레트 열기 (미리보기 전용)" $(, $arg)*),
        5 => format!("RGB: {0}, {1}, {2}\nAlpha: {3}\nKlicken zum Öffnen der Palette (nur Vorschau)" $(, $arg)*),
        6 => format!("RGB: {0}, {1}, {2}\nAlpha: {3}\nCliquer pour ouvrir la palette (aperçu uniquement)" $(, $arg)*),
        _ => format!("RGB: {0}, {1}, {2}\nAlpha: {3}\n点击打开调色板（仅预览）" $(, $arg)*),
    }};
    ("占用空间：{0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("佔用空間：{0}" $(, $arg)*),
        2 => format!("Space used: {0}" $(, $arg)*),
        3 => format!("使用容量：{0}" $(, $arg)*),
        4 => format!("사용 공간: {0}" $(, $arg)*),
        5 => format!("Belegter Speicher: {0}" $(, $arg)*),
        6 => format!("Espace utilisé : {0}" $(, $arg)*),
        _ => format!("占用空间：{0}" $(, $arg)*),
    }};
    ("已释放：{0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("已釋放：{0}" $(, $arg)*),
        2 => format!("Freed: {0}" $(, $arg)*),
        3 => format!("解放済み：{0}" $(, $arg)*),
        4 => format!("확보된 공간: {0}" $(, $arg)*),
        5 => format!("Freigegeben: {0}" $(, $arg)*),
        6 => format!("Libéré : {0}" $(, $arg)*),
        _ => format!("已释放：{0}" $(, $arg)*),
    }};
    ("旧缓存：{0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("舊快取：{0}" $(, $arg)*),
        2 => format!("Legacy cache: {0}" $(, $arg)*),
        3 => format!("旧キャッシュ：{0}" $(, $arg)*),
        4 => format!("이전 캐시: {0}" $(, $arg)*),
        5 => format!("Alter Cache: {0}" $(, $arg)*),
        6 => format!("Ancien cache : {0}" $(, $arg)*),
        _ => format!("旧缓存：{0}" $(, $arg)*),
    }};
    ("缓存操作失败：{0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("快取操作失敗：{0}" $(, $arg)*),
        2 => format!("Cache operation failed: {0}" $(, $arg)*),
        3 => format!("キャッシュ操作に失敗：{0}" $(, $arg)*),
        4 => format!("캐시 작업 실패: {0}" $(, $arg)*),
        5 => format!("Cache-Vorgang fehlgeschlagen: {0}" $(, $arg)*),
        6 => format!("Échec de l’opération sur le cache : {0}" $(, $arg)*),
        _ => format!("缓存操作失败：{0}" $(, $arg)*),
    }};
    ("播放位置记录保存失败：{0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("播放位置記錄儲存失敗：{0}" $(, $arg)*),
        2 => format!("Could not save playback position: {0}" $(, $arg)*),
        3 => format!("再生位置を保存できません：{0}" $(, $arg)*),
        4 => format!("재생 위치 저장 실패: {0}" $(, $arg)*),
        5 => format!("Wiedergabeposition konnte nicht gespeichert werden: {0}" $(, $arg)*),
        6 => format!("Impossible d’enregistrer la position de lecture : {0}" $(, $arg)*),
        _ => format!("播放位置记录保存失败：{0}" $(, $arg)*),
    }};
    ("已复制后缀 {0}，请在系统默认应用页面搜索；返回此窗口会刷新状态。" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("已複製副檔名 {0}，請在系統預設應用程式頁面搜尋；返回此視窗會重新整理狀態。" $(, $arg)*),
        2 => format!("Copied {0}; search for it in Default apps. Returning to this window refreshes the status." $(, $arg)*),
        3 => format!("{0} をコピーしました。既定のアプリで検索してください。このウィンドウに戻ると状態を更新します。" $(, $arg)*),
        4 => format!("{0} 복사됨. 기본 앱에서 검색하세요. 이 창으로 돌아오면 상태가 새로 고쳐집니다." $(, $arg)*),
        5 => format!("{0} kopiert. Suchen Sie in den Standard-Apps danach. Bei Rückkehr wird der Status aktualisiert." $(, $arg)*),
        6 => format!("{0} copiée. Recherchez-la dans les applications par défaut. Le retour à cette fenêtre actualise l’état." $(, $arg)*),
        _ => format!("已复制后缀 {0}，请在系统默认应用页面搜索；返回此窗口会刷新状态。" $(, $arg)*),
    }};
    ("无法打开系统设置或读取关联：{0}" $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {
        1 => format!("無法開啟系統設定或讀取關聯：{0}" $(, $arg)*),
        2 => format!("Could not open settings or read associations: {0}" $(, $arg)*),
        3 => format!("設定を開くか関連付けを確認できません：{0}" $(, $arg)*),
        4 => format!("설정을 열거나 연결을 읽을 수 없음: {0}" $(, $arg)*),
        5 => format!("Einstellungen oder Zuordnungen konnten nicht geöffnet bzw. gelesen werden: {0}" $(, $arg)*),
        6 => format!("Impossible d’ouvrir les paramètres ou de lire les associations : {0}" $(, $arg)*),
        _ => format!("无法打开系统设置或读取关联：{0}" $(, $arg)*),
    }};
}
